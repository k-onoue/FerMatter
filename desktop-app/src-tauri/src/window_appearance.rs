use crate::files::Result;
use serde_json::Value;

#[cfg(windows)]
mod menu {
    use crate::files::{err, Result};
    use std::cell::Cell;
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        UI::{
            Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
            WindowsAndMessaging::{
                DrawMenuBar, WM_NCACTIVATE, WM_NCDESTROY, WM_NCPAINT, WM_SIZE, WM_THEMECHANGED,
            },
        },
    };

    const SUBCLASS_ID: usize = 0x464d4150;
    thread_local! {
        static REDRAWING: Cell<bool> = const { Cell::new(false) };
    }

    unsafe extern "system" fn redraw_after_native_frame(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        subclass: usize,
        _: usize,
    ) -> LRESULT {
        if message == WM_NCDESTROY {
            unsafe { RemoveWindowSubclass(hwnd, Some(redraw_after_native_frame), subclass) };
        }
        let result = unsafe { DefSubclassProc(hwnd, message, wparam, lparam) };
        if matches!(
            message,
            WM_NCACTIVATE | WM_NCPAINT | WM_SIZE | WM_THEMECHANGED
        ) {
            // Windows can repaint the bar with its system brush after muda has
            // applied the app theme. Redraw the whole menu after that frame pass,
            // so its existing themed renderer also handles focus/resize changes.
            // DrawMenuBar may synchronously repaint the non-client area again.
            // Cover that final frame pass without recursively requesting redraws.
            REDRAWING.with(|redrawing| {
                if !redrawing.replace(true) {
                    unsafe { DrawMenuBar(hwnd) };
                    redrawing.set(false);
                }
            });
        }
        result
    }

    pub fn install(window: &tauri::WebviewWindow) -> Result<()> {
        let hwnd = window.hwnd().map_err(err)?.0;
        // apply() runs on the window's UI thread. Reinstalling the same callback
        // and ID updates one subclass; WM_NCDESTROY removes it before destruction.
        if unsafe { SetWindowSubclass(hwnd, Some(redraw_after_native_frame), SUBCLASS_ID, 0) } == 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
        unsafe { DrawMenuBar(hwnd) };
        Ok(())
    }
}

#[cfg(not(target_os = "macos"))]
fn colors(value: &Value) -> Result<(bool, tauri::window::Color)> {
    let dark = value["dark"]
        .as_bool()
        .ok_or("Expected an appearance mode.")?;
    let hex = value["background"]
        .as_str()
        .and_then(|value| value.strip_prefix('#'))
        .filter(|value| value.len() == 6 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or("Expected a six-digit window background color.")?;
    let channel = |start| u8::from_str_radix(&hex[start..start + 2], 16).unwrap();
    Ok((
        dark,
        tauri::window::Color(channel(0), channel(2), channel(4), 255),
    ))
}

pub fn apply(app: &tauri::AppHandle, label: &str, value: &Value) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let _ = (app, label, value);
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        use crate::{files::err, lifecycle};
        use tauri::Manager;

        let (dark, background) = colors(value)?;
        let label = label.to_owned();
        lifecycle::on_main(app, move |app| {
            let window = app.get_webview_window(&label).ok_or("Window is closed.")?;
            // Paint the native window and webview with the same opaque palette
            // color; otherwise uncovered frame/menu areas retain the OS brush.
            window.set_background_color(Some(background)).map_err(err)?;
            // Tauri also propagates this theme to the Windows native menu.
            // On Linux it updates the application's GTK theme preference.
            window
                .set_theme(Some(if dark {
                    tauri::Theme::Dark
                } else {
                    tauri::Theme::Light
                }))
                .map_err(err)?;
            #[cfg(windows)]
            menu::install(&window)?;
            Ok(())
        })
    }
}

#[cfg(all(test, not(target_os = "macos")))]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn native_background_is_opaque_and_uses_the_palette_color() {
        let (dark, color) = colors(&json!({"dark":true,"background":"#1d232C"})).unwrap();
        assert!(dark);
        assert_eq!((color.0, color.1, color.2, color.3), (29, 35, 44, 255));
        for background in [
            "transparent",
            "#123",
            "#12345600",
            "#zzzzzz",
            "#１２３４５６",
        ] {
            assert!(colors(&json!({"dark":false,"background":background})).is_err());
        }
        assert!(colors(&json!({"background":"#ffffff"})).is_err());
    }
}
