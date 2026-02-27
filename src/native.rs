use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt as _;
use std::path::PathBuf;

use euclid::default::Size2D;

use windows::core::*;
use windows::Win32::{
    Foundation::*,
    Graphics::Dwm::*,
    Graphics::Gdi::*,
    System::Threading::*,
    UI::WindowsAndMessaging::*,
};

pub fn enumerate_windows() -> Result<Vec<HWND>> {
    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: `lparam` carries a pointer to a stack-local `Vec<HWND>` created
        // in `enumerate_windows()` below. The pointer is non-null, properly aligned,
        // and valid for the entire synchronous duration of `EnumWindows`. No aliasing
        // occurs because the callback is invoked sequentially — each `&mut Vec` exists
        // only within a single callback invocation.
        unsafe {
            (lparam.0 as *mut Vec<HWND>)
                .as_mut_unchecked()
                .push(hwnd);
            TRUE
        }
    }

    let mut out = Vec::new();
    let out_ptr = &raw mut out;

    // SAFETY: The callback has the correct `extern "system"` ABI and signature.
    // `LPARAM` carries a valid pointer to `out`, which lives on the stack and
    // outlives the synchronous `EnumWindows` call.
    unsafe { EnumWindows(Some(enum_proc), LPARAM(out_ptr as _)) }.map(|()| out)
}

/// Checks whether a window is "cloaked" (hidden by DWM).
/// Cloaked windows are technically visible but not shown to the user — common
/// with UWP app placeholders and windows on other virtual desktops.
pub fn is_cloaked(hwnd: HWND) -> bool {
    let mut cloaked: u32 = 0;
    let cloaked_ptr = &raw mut cloaked;

    // SAFETY: `cloaked` is a stack-local `u32`; its raw pointer is valid and
    // properly aligned. The buffer size (`size_of::<u32>()`) matches the type
    // expected by `DWMWA_CLOAKED`.
    let hr = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            cloaked_ptr.cast(),
            size_of::<u32>() as u32)
    };
    hr.is_ok() && cloaked != 0
}

/// Returns the client-area size `(width, height)` of a window, or `(0, 0)` on failure.
///
/// Uses `GetClientRect` because Windows Graphics Capture captures the client area,
/// so these dimensions match the captured texture size.
pub fn get_client_size(hwnd: HWND) -> Result<Size2D<u32>> {
    let mut rect = RECT::default();
    // SAFETY: `hwnd` is a valid enumerated handle; `&raw mut rect` is a valid local.
    unsafe { GetClientRect(hwnd, &raw mut rect) }?;
    Ok(Size2D::new(
        (rect.right - rect.left) as u32,
        (rect.bottom - rect.top) as u32))
}

/// Returns the window title as a `String`, or an empty string on failure.
///
/// Uses `GetWindowTextLengthW` and `GetWindowTextW` to retrieve the title as UTF-16,
/// then converts it to a Rust `String`. The conversion is lossy and replaces invalid
/// UTF-16 sequences with the Unicode replacement character, but this is acceptable
/// for display purposes.
pub fn get_window_text(hwnd: HWND) -> String {
    // SAFETY: Simple query with no pointer arguments beyond `hwnd`.
    let buf_len = unsafe { GetWindowTextLengthW(hwnd) } as usize + 1;
    let mut buf = vec![0u16; buf_len];

    // SAFETY: `hwnd` is a valid enumerated handle; `&mut buf` is a valid
    // buffer of `u16`, and `GetWindowTextW` writes at most `buf_len`
    // elements including the null terminator.
    let len = unsafe { GetWindowTextW(hwnd, &mut buf) } as usize;
    OsString::from_wide(&buf[..len])
        .to_string_lossy()
        .into_owned()
}

/// Returns the process ID of the window's owning process, or `0` on failure
/// (e.g. elevated process).
pub fn get_process_id(hwnd: HWND) -> u32 {
    let mut pid = 0;
    // SAFETY: `hwnd` is a valid enumerated handle; `&raw mut pid` is a valid local.
    unsafe { GetWindowThreadProcessId(hwnd, Some(&raw mut pid)); }
    pid
}

/// Returns the full executable path of a process given its ID, or `None` on failure
/// (e.g. elevated process, system process, or process that has already exited).
pub fn get_executable_path(pid: u32) -> Option<PathBuf> {
    // SAFETY: `pid` is a non-zero process ID obtained from `GetWindowThreadProcessId`.
    // `OpenProcess` with `QUERY_LIMITED_INFORMATION` is a low-privilege operation.
    // `buf` is a stack-allocated 260-element u16 array (MAX_PATH). `CloseHandle` is
    // always called on the opened handle before returning.
    #[expect(clippy::multiple_unsafe_ops_per_block, reason = "Windows API calls")]
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        if handle.is_invalid() {
            None?;
        }

        let mut buf = [0u16; 260];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &raw mut len);
        let _ = CloseHandle(handle);
        ok.ok()?;

        Some(PathBuf::from(OsString::from_wide(&buf[..len as usize])))
    }
}

/// Returns the [`MONITORINFO`] of the monitor that a window is currently on,
/// or `None` if it cannot be determined.
pub fn get_monitor_info_from_window(hwnd: HWND) -> Option<MONITORINFO> {
    // SAFETY: `MONITOR_DEFAULTTOPRIMARY` guarantees a valid `HMONITOR` is
    // returned, falling back to the primary monitor.
    let hmonitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY) };
    let mut monitor_info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as _,
        ..Default::default()
    };

    // SAFETY: `monitor_info` is stack-local with `cbSize` correctly initialized
    // to `size_of::<MONITORINFO>()`; its raw pointer is valid for the call.
    unsafe { GetMonitorInfoW(hmonitor, &raw mut monitor_info) }
        .as_bool()
        .then_some(monitor_info)
}

pub fn resize_client(hwnd: HWND, width: i32, height: i32) -> Result<()> {
    let mut window_rect = RECT::default();
    let mut client_rect = RECT::default();

    // SAFETY: Both `RECT`s are stack-local; their raw pointers are valid for
    // the duration of each call.
    unsafe { GetWindowRect(hwnd, &raw mut window_rect) }?;
    // SAFETY: Mentioned above;
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

    // SAFETY: All positional/size arguments are computed from prior successful
    // Win32 API calls; flag constants are valid.
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
