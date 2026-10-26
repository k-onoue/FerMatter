use crate::files::Result;
use serde_json::Value;

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
                .map_err(err)
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
