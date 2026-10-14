fn main() {
    println!("cargo:rerun-if-changed=assets/app.ico");
    compile_windows_resources();
}

#[cfg(target_os = "windows")]
fn compile_windows_resources() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/app.ico");
        resource.compile().expect("compile Windows resources");
    }
}

#[cfg(not(target_os = "windows"))]
fn compile_windows_resources() {}
