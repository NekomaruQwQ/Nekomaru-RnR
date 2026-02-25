use windows::core::*;
use windows::Win32::{
    Foundation::*,
    Graphics::Dwm::*,
    Graphics::Gdi::*,
    UI::WindowsAndMessaging::*,
};

pub fn enumerate_windows() -> Result<Vec<HWND>> {
    unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            (lparam.0 as *mut Vec<HWND>)
                .as_mut_unchecked()
                .push(hwnd);
            TRUE
        }
    }

    let mut out = Vec::new();
    unsafe {
        EnumWindows(
            Some(enum_windows_proc),
            LPARAM(&raw mut out as _))
    }?;
    Ok(out)
}

/// Checks whether a window is "cloaked" (hidden by DWM).
/// Cloaked windows are technically visible but not shown to the user — common
/// with UWP app placeholders and windows on other virtual desktops.
pub fn is_hidden_by_dwm(hwnd: HWND) -> bool {
    let mut cloaked: u32 = 0;
    let cloacked_ptr = &raw mut cloaked;
    let hr = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            cloacked_ptr.cast(),
            size_of::<u32>() as u32)
    };
    hr.is_ok() && cloaked != 0
}

pub fn get_client_size(hwnd: HWND) -> Result<SIZE> {
    let mut rect = RECT::default();
    unsafe { GetClientRect(hwnd, &raw mut rect) }?;
    Ok(SIZE {
        cx: rect.right  - rect.left,
        cy: rect.bottom - rect.top,
    })
}

pub fn get_window_text(hwnd: HWND) -> String {
    let buf_len = unsafe { GetWindowTextLengthW(hwnd) } as usize + 1;
    let mut buf = vec![0u16; buf_len];
    let _ = unsafe { GetWindowTextW(hwnd, &mut buf) };
    if let Some(pos) = buf.iter().position(|&c| c == 0) {
        buf.truncate(pos);
    }
    String::from_utf16_lossy(&buf)
}

pub fn get_monitor_info_from_window(hwnd: HWND) -> Option<MONITORINFO> {
    let hmonitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY) };
    let mut monitor_info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as _,
        ..Default::default()
    };
    unsafe { GetMonitorInfoW(hmonitor, &raw mut monitor_info) }
        .as_bool()
        .then_some(monitor_info)
}

pub fn resize_client(hwnd: HWND, width: i32, height: i32) -> Result<()> {
    let mut window_rect = RECT::default();
    let mut client_rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &raw mut window_rect) }?;
    unsafe { GetClientRect(hwnd, &raw mut client_rect) }?;
    let old_window_size = SIZE {
        cx: window_rect.right  - window_rect.left,
        cy: window_rect.bottom - window_rect.top,
    };
    let old_client_size = SIZE {
        cx: client_rect.right  - client_rect.left,
        cy: client_rect.bottom - client_rect.top,
    };
    let new_client_size = SIZE {
        cx: width,
        cy: height,
    };
    let new_window_size = SIZE {
        cx: old_window_size.cx + new_client_size.cx - old_client_size.cx,
        cy: old_window_size.cy + new_client_size.cy - old_client_size.cy,
    };
    let new_x = window_rect.left + old_window_size.cx / 2i32 - new_window_size.cx / 2i32;
    let new_y = window_rect.top  + old_window_size.cy / 2i32 - new_window_size.cy / 2i32;
    unsafe {
        SetWindowPos(
            hwnd,
            None,
            new_x,
            new_y,
            new_window_size.cx,
            new_window_size.cy,
            SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_NOZORDER)
    }
}
