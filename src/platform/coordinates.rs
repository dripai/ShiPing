use crate::domain::Bounds;
use anyhow::{Result, anyhow};

/// Convert physical pixels to display-relative points, retaining negative
/// margins for indicators drawn just outside the capture rectangle.
pub(super) fn relative_points(
    bounds: Bounds,
    display: Bounds,
    logical_size: (f64, f64),
) -> Result<(f64, f64, f64, f64)> {
    let scale_x = display.width as f64 / logical_size.0;
    let scale_y = display.height as f64 / logical_size.1;
    if !scale_x.is_finite() || !scale_y.is_finite() || scale_x <= 0. || scale_y <= 0. {
        return Err(anyhow!("Invalid display scale"));
    }
    Ok((
        (i64::from(bounds.left) - i64::from(display.left)) as f64 / scale_x,
        (i64::from(bounds.top) - i64::from(display.top)) as f64 / scale_y,
        bounds.width as f64 / scale_x,
        bounds.height as f64 / scale_y,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retina_overlay_is_display_relative_and_keeps_outside_border() {
        let display = Bounds {
            left: 1920,
            top: 0,
            width: 2880,
            height: 1800,
        };
        let outer = Bounds {
            left: 1917,
            top: -35,
            width: 2886,
            height: 1838,
        };
        assert_eq!(
            relative_points(outer, display, (1440., 900.)).unwrap(),
            (-1.5, -17.5, 1443., 919.)
        );
    }
    #[test]
    fn adjacent_displays_use_their_own_scale() {
        let first = Bounds {
            left: 0,
            top: 0,
            width: 1920,
            height: 1080,
        };
        let second = Bounds {
            left: 1920,
            top: 0,
            width: 2880,
            height: 1800,
        };
        assert_eq!(
            relative_points(first, first, (1920., 1080.)).unwrap(),
            (0., 0., 1920., 1080.)
        );
        assert_eq!(
            relative_points(second, second, (1440., 900.)).unwrap(),
            (0., 0., 1440., 900.)
        );
        assert!(relative_points(second, second, (0., 900.)).is_err());
    }
}
