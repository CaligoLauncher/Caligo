//! A real OS caption, not a client-side approximation.
pub type ApplyTitlebar = Box<dyn FnOnce() -> Result<(), String>>;
#[cfg(target_os = "windows")]
pub fn prepare_native_titlebar(
    window: &gpui::Window,
) -> Result<ApplyTitlebar, String> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::{
        Foundation::{GetLastError, HWND, SetLastError, WIN32_ERROR},
        UI::WindowsAndMessaging::*,
    };

    let handle = HasWindowHandle::window_handle(window).map_err(|e| e.to_string())?;
    let RawWindowHandle::Win32(raw) = handle.as_raw() else {
        return Err("Окно не предоставляет Win32 handle.".into());
    };
    let address = raw.hwnd.get();
    Ok(Box::new(move || {
    let hwnd = HWND(address as *mut std::ffi::c_void);

    // GPUI 0.2.2 defers non-client layout/hit testing when
    // appears_transparent=false, but its normal creation style omits WS_CAPTION.
    // Preserve all other style bits and let DefWindowProc draw/operate the caption.
    // SAFETY: called once on the UI executor before user input can close the
    // newly created window. No App borrow is held: SetWindowPos can synchronously
    // re-enter GPUI resize callbacks.
    unsafe {
        let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
        if style & WS_CAPTION.0 == WS_CAPTION.0 {
            return Ok(());
        }
        SetLastError(WIN32_ERROR(0));
        let previous = SetWindowLongW(hwnd, GWL_STYLE, (style | WS_CAPTION.0) as i32);
        if previous == 0 && GetLastError().0 != 0 {
            return Err(windows::core::Error::from_win32().to_string());
        }
        SetWindowPos(
            hwnd,
            None,
            0, 0, 0, 0,
            SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        ).map_err(|e| e.to_string())?;
        if GetWindowLongW(hwnd, GWL_STYLE) as u32 & WS_CAPTION.0 != WS_CAPTION.0 {
            return Err("Windows не применила системную строку заголовка.".into());
        }
    }
    Ok(())
    }))
}

#[cfg(not(target_os = "windows"))]
pub fn prepare_native_titlebar(
    _window: &gpui::Window,
) -> Result<ApplyTitlebar, String> {
    Ok(Box::new(|| Ok(())))
}