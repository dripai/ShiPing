use anyhow::{Context, Result};
use log::{Level, LevelFilter, Log, Metadata, Record};
use std::{
    backtrace::Backtrace,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

struct FileLogger {
    file: Mutex<File>,
}

impl FileLogger {
    fn create(path: &Path) -> io::Result<Self> {
        Ok(Self {
            file: Mutex::new(OpenOptions::new().create_new(true).write(true).open(path)?),
        })
    }

    fn write(&self, record: &Record<'_>) -> io::Result<()> {
        // Serialize before taking the file lock, including user-provided Display
        // implementations, so a panic while formatting can still reach the hook.
        let thread = std::thread::current();
        let entry = serde_json::json!({
            "time_unix_ms": timestamp_millis(),
            "pid": std::process::id(),
            "thread": thread.name().unwrap_or("unnamed"),
            "thread_id": format!("{:?}", thread.id()),
            "level": record.level().as_str(),
            "target": record.target(),
            "file": record.file(),
            "line": record.line(),
            "message": record.args().to_string(),
        });
        let mut bytes = serde_json::to_vec(&entry)?;
        bytes.push(b'\n');
        self.file
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .write_all(&bytes)
    }
}

impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record<'_>) {
        if self.enabled(record.metadata())
            && let Err(error) = self.write(record)
        {
            let _ = writeln!(io::stderr(), "ShiPing log write failed: {error}");
        }
    }

    fn flush(&self) {
        if let Err(error) = self
            .file
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .sync_data()
        {
            let _ = writeln!(io::stderr(), "ShiPing log flush failed: {error}");
        }
    }
}

fn timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

pub(crate) fn initialize(directory: &Path) -> Result<PathBuf> {
    fs::create_dir_all(directory)
        .with_context(|| format!("Failed to create log directory: {}", directory.display()))?;
    let path = directory.join(format!(
        "shiping-{}-{}.log",
        timestamp_millis(),
        std::process::id()
    ));
    let logger = FileLogger::create(&path)
        .with_context(|| format!("Failed to create log file: {}", path.display()))?;
    log::set_boxed_logger(Box::new(logger)).context("Failed to install ShiPing logger")?;
    log::set_max_level(LevelFilter::Info);

    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!(target: "panic", "{info}\nBacktrace:\n{}", Backtrace::force_capture());
        log::logger().flush();
        previous(info);
    }));
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{process::Command, sync::Arc, thread};

    fn temporary_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "shiping-log-test-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn utf8_and_multiline_messages_form_one_json_record() {
        let path = temporary_path("utf8");
        let logger = FileLogger::create(&path).unwrap();
        logger.log(
            &Record::builder()
                .level(Level::Error)
                .target("selection")
                .args(format_args!("区域选择\n错误详情"))
                .build(),
        );
        logger.flush();
        drop(logger);
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content.lines().count(), 1);
        let entry: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(entry["message"], "区域选择\n错误详情");
        assert_eq!(entry["level"], "ERROR");
        assert_eq!(entry["target"], "selection");
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn concurrent_writes_keep_complete_records() {
        let path = temporary_path("threads");
        let logger = Arc::new(FileLogger::create(&path).unwrap());
        let workers = (0..4)
            .map(|worker| {
                let logger = logger.clone();
                thread::spawn(move || {
                    for index in 0..20 {
                        logger.log(
                            &Record::builder()
                                .level(Level::Info)
                                .target("test")
                                .args(format_args!("{worker}:{index}"))
                                .build(),
                        );
                    }
                })
            })
            .collect::<Vec<_>>();
        for worker in workers {
            worker.join().unwrap();
        }
        logger.flush();
        drop(logger);
        let content = fs::read_to_string(&path).unwrap();
        let records = content
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(records.len(), 80);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn panic_backtrace_is_saved() {
        const PROBE: &str = "SHIPING_LOGGING_TEST_DIRECTORY";
        if let Some(directory) = std::env::var_os(PROBE) {
            initialize(Path::new(&directory)).unwrap();
            panic!("intentional panic log verification");
        }
        let directory = temporary_path("panic");
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "logging::tests::panic_backtrace_is_saved",
                "--nocapture",
            ])
            .env(PROBE, &directory)
            .output()
            .unwrap();
        assert!(!output.status.success());
        let entries = fs::read_dir(&directory)
            .unwrap()
            .collect::<io::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(entries.len(), 1);
        let path = entries[0].path();
        let content = fs::read_to_string(&path).unwrap();
        let entry: serde_json::Value =
            serde_json::from_str(content.lines().next().unwrap()).unwrap();
        let message = entry["message"].as_str().unwrap();
        assert_eq!(entry["target"], "panic");
        assert!(message.contains("intentional panic log verification"));
        assert!(message.contains("Backtrace:"));
        assert!(message.contains("logging::initialize"));
        fs::remove_file(path).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}
