//! Tray icon management for the menubar.

use thiserror::Error;
use tray_icon::Icon;

#[derive(Error, Debug)]
pub enum TrayError {
    #[error("Failed to load icon: {0}")]
    IconLoad(#[from] tray_icon::BadIcon),
    #[error("Failed to decode image")]
    ImageDecode,
}

/// Loads the tray icon from embedded resources or generates a default one.
pub fn load_icon() -> Result<Icon, TrayError> {
    // Try to load from embedded PNG if available
    // For now, generate a simple colored icon programmatically

    // Create a 22x22 tomato-colored icon (standard macOS tray icon size)
    let size = 22u32;
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);

    // Create a simple tomato-shaped icon
    let center = size as f32 / 2.0;
    let radius = (size as f32 / 2.0) - 2.0;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance <= radius {
                // Inside the tomato - red color
                rgba.push(220); // R
                rgba.push(50); // G
                rgba.push(47); // B
                rgba.push(255); // A
            } else if distance <= radius + 1.0 {
                // Anti-aliased edge
                let alpha = ((radius + 1.0 - distance) * 255.0) as u8;
                rgba.push(220);
                rgba.push(50);
                rgba.push(47);
                rgba.push(alpha);
            } else {
                // Outside - transparent
                rgba.push(0);
                rgba.push(0);
                rgba.push(0);
                rgba.push(0);
            }
        }
    }

    // Add a small green stem at the top
    let stem_start = 2;
    let stem_end = 5;
    let stem_width = 3;
    let stem_center = size / 2;

    for y in stem_start..stem_end {
        for x in (stem_center - stem_width / 2)..=(stem_center + stem_width / 2) {
            let idx = ((y * size + x) * 4) as usize;
            if idx + 3 < rgba.len() {
                rgba[idx] = 76; // R (green)
                rgba[idx + 1] = 153; // G
                rgba[idx + 2] = 0; // B
                rgba[idx + 3] = 255; // A
            }
        }
    }

    Icon::from_rgba(rgba, size, size).map_err(TrayError::IconLoad)
}

/// Creates a template icon suitable for macOS dark/light mode.
/// Template icons should be grayscale and the system will tint them appropriately.
#[allow(dead_code)]
pub fn load_template_icon() -> Result<Icon, TrayError> {
    let size = 22u32;
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);

    let center = size as f32 / 2.0;
    let radius = (size as f32 / 2.0) - 2.0;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance <= radius {
                // Inside - black (will be tinted by macOS)
                rgba.push(0);
                rgba.push(0);
                rgba.push(0);
                rgba.push(255);
            } else if distance <= radius + 1.0 {
                // Anti-aliased edge
                let alpha = ((radius + 1.0 - distance) * 255.0) as u8;
                rgba.push(0);
                rgba.push(0);
                rgba.push(0);
                rgba.push(alpha);
            } else {
                // Outside - transparent
                rgba.push(0);
                rgba.push(0);
                rgba.push(0);
                rgba.push(0);
            }
        }
    }

    Icon::from_rgba(rgba, size, size).map_err(TrayError::IconLoad)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_icon() {
        let icon = load_icon();
        assert!(icon.is_ok());
    }

    #[test]
    fn test_load_template_icon() {
        let icon = load_template_icon();
        assert!(icon.is_ok());
    }
}
