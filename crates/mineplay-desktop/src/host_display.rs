use mineplay_android_shell::DisplaySize;
use mineplay_config::AppConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayTarget {
    pub size: DisplaySize,
    pub dynamic: bool,
}

#[must_use]
pub fn resolve_display_target(config: &AppConfig) -> DisplayTarget {
    if config.playback.dynamic_display
        && let Some(host) = primary_display_size()
    {
        return DisplayTarget {
            size: fit_size_to_caps(
                host,
                DisplaySize {
                    width: config.video.preferred_width.max(2),
                    height: config.video.preferred_height.max(2),
                },
            ),
            dynamic: true,
        };
    }

    DisplayTarget {
        size: fit_aspect_to_caps(
            config.playback.target_aspect_width.max(1),
            config.playback.target_aspect_height.max(1),
            DisplaySize {
                width: config.video.preferred_width.max(2),
                height: config.video.preferred_height.max(2),
            },
        ),
        dynamic: false,
    }
}

#[must_use]
pub fn primary_display_size() -> Option<DisplaySize> {
    #[cfg(windows)]
    {
        use std::mem::size_of;
        use windows_sys::Win32::Graphics::Gdi::{
            DEVMODEW, ENUM_CURRENT_SETTINGS, EnumDisplaySettingsW,
        };
        use windows_sys::Win32::UI::{
            HiDpi::GetDpiForSystem,
            WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN},
        };

        let mut dev_mode = unsafe { std::mem::zeroed::<DEVMODEW>() };
        dev_mode.dmSize = size_of::<DEVMODEW>() as u16;
        if unsafe { EnumDisplaySettingsW(std::ptr::null(), ENUM_CURRENT_SETTINGS, &mut dev_mode) }
            != 0
        {
            let width = dev_mode.dmPelsWidth;
            let height = dev_mode.dmPelsHeight;
            if width > 1 && height > 1 {
                return Some(normalize_even_size(DisplaySize { width, height }));
            }
        }

        let logical_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let logical_height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
        let dpi = unsafe { GetDpiForSystem() }.max(96);
        let width = (logical_width.max(2) as u32 * dpi) / 96;
        let height = (logical_height.max(2) as u32 * dpi) / 96;
        if width > 1 && height > 1 {
            return Some(normalize_even_size(DisplaySize { width, height }));
        }
    }

    None
}

#[must_use]
pub fn fit_size_to_caps(size: DisplaySize, caps: DisplaySize) -> DisplaySize {
    let width = size.width.max(2) as f64;
    let height = size.height.max(2) as f64;
    let width_scale = caps.width.max(2) as f64 / width;
    let height_scale = caps.height.max(2) as f64 / height;
    let scale = width_scale.min(height_scale).min(1.0);
    normalize_even_size(DisplaySize {
        width: (width * scale).round().max(2.0) as u32,
        height: (height * scale).round().max(2.0) as u32,
    })
}

#[must_use]
pub fn fit_aspect_to_caps(aspect_width: u32, aspect_height: u32, caps: DisplaySize) -> DisplaySize {
    let aspect_width = aspect_width.max(1) as f64;
    let aspect_height = aspect_height.max(1) as f64;
    let width_scale = caps.width.max(2) as f64 / aspect_width;
    let height_scale = caps.height.max(2) as f64 / aspect_height;
    let scale = width_scale.min(height_scale);

    normalize_even_size(DisplaySize {
        width: (aspect_width * scale).round() as u32,
        height: (aspect_height * scale).round() as u32,
    })
}

#[must_use]
pub fn normalize_even_size(size: DisplaySize) -> DisplaySize {
    let width = size.width.max(2) & !1;
    let height = size.height.max(2) & !1;
    DisplaySize {
        width: width.max(2),
        height: height.max(2),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fits_host_size_into_preferred_caps() {
        let size = fit_size_to_caps(
            DisplaySize {
                width: 2560,
                height: 1440,
            },
            DisplaySize {
                width: 1920,
                height: 1080,
            },
        );

        assert_eq!(
            size,
            DisplaySize {
                width: 1920,
                height: 1080,
            }
        );
    }

    #[test]
    fn fits_custom_aspect_into_caps() {
        let size = fit_aspect_to_caps(
            21,
            9,
            DisplaySize {
                width: 1920,
                height: 1080,
            },
        );

        assert_eq!(
            size,
            DisplaySize {
                width: 1920,
                height: 822,
            }
        );
    }
}
