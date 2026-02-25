use windows::core::*;
use windows::Win32::{
    Foundation::*,
    UI::WindowsAndMessaging::*,
};

use crate::native::*;

pub const RESOLUTION_GROUPS: &[(&str, &[SIZE])] = &[
    ("16:10", RESOLUTIONS_16_10),
];

pub const RESOLUTIONS_16_10: &[SIZE] = &[
    SIZE { cx:  480, cy:  300 },
    SIZE { cx:  640, cy:  400 },
    SIZE { cx:  800, cy:  500 },
    SIZE { cx:  960, cy:  600 },
    SIZE { cx: 1280, cy:  800 },
    SIZE { cx: 1440, cy:  900 },
    SIZE { cx: 1680, cy: 1050 },
    SIZE { cx: 1920, cy: 1200 },
    SIZE { cx: 2560, cy: 1600 },
    SIZE { cx: 2880, cy: 1800 },
    SIZE { cx: 3840, cy: 2400 },
];

pub fn is_known_resolution(size: SIZE) -> bool {
    RESOLUTION_GROUPS
        .iter()
        .flat_map(|&(_, arr)| arr)
        .any(|&item| item == size)
}

pub const fn get_center_of_rect(rect: &RECT) -> POINT {
    POINT {
        x: rect.left + (rect.right  - rect.left) / 2,
        y: rect.top  + (rect.bottom - rect.top ) / 2,
    }
}

/// Checks if a window is active and should be included in the list of windows
/// that can be manipulated by the user. This function filters out windows that
/// are not visible, minimized, maximized, owned by other windows, or cloaked by
/// the Desktop Window Manager (DWM).
pub fn is_active(hwnd: HWND) -> bool {
    // SAFETY: `IsWindowVisible`, `IsIconic`, `IsZoomed`, and `GetWindow` are
    // simple boolean/handle queries on `hwnd` with no pointer arguments.
    #[expect(clippy::multiple_unsafe_ops_per_block, reason = "Windows API calls")]
    unsafe {
        IsWindowVisible(hwnd).as_bool()
        && !IsIconic(hwnd).as_bool()
        && !IsZoomed(hwnd).as_bool()
        // Exclude owned windows, which are typically tooltips, popups, and other
        // auxiliary windows that shouldn't be treated as main application windows.
        && GetWindow(hwnd, GW_OWNER)
            .unwrap_or_default()
            .is_invalid()
        // Exclude cloaked windows, which are technically visible but not shown to
        // the user.
        && !is_hidden_by_dwm(hwnd)
    }
}

pub fn is_centered(hwnd: HWND) -> Option<bool> {
    let monitor_info = get_monitor_info_from_window(hwnd)?;
    let mut window_rect = RECT::default();

    // SAFETY: `window_rect` is stack-local; its raw pointer is valid
    // for the duration of the call.
    unsafe { GetWindowRect(hwnd, &raw mut window_rect) }.ok()?;
    let screen_center = get_center_of_rect(&monitor_info.rcWork);
    let window_center = get_center_of_rect(&window_rect);
    Some(window_center == screen_center)
}

pub fn center_to_screen(hwnd: HWND) -> Result<()> {
    let Some(monitor_info) = get_monitor_info_from_window(hwnd) else {
        return Err(Error::empty());
    };
    let mut window_rect = RECT::default();
    // SAFETY: `window_rect` is stack-local; its raw pointer is valid for the
    // duration of the call.
    unsafe { GetWindowRect(hwnd, &raw mut window_rect) }?;
    // SAFETY: Positional arguments are computed from prior successful API
    // calls (`get_monitor_info_from_window`, `GetWindowRect`); `SWP_NOSIZE`
    // makes the width/height arguments (0, 0) ignored; flag constants are valid.
    unsafe {
        SetWindowPos(
            hwnd,
            None,
            monitor_info.rcWork.left
                + (monitor_info.rcWork.right - monitor_info.rcWork.left) / 2
                - (window_rect.right - window_rect.left) / 2,
            monitor_info.rcWork.top
                + (monitor_info.rcWork.bottom - monitor_info.rcWork.top) / 2
                - (window_rect.bottom - window_rect.top) / 2,
            0,
            0,
            SWP_NOACTIVATE |
            SWP_NOOWNERZORDER |
            SWP_NOSIZE |
            SWP_NOZORDER)
    }
}
