//! Full-screen Spaces may have no desktop behind a transparent window. Resolve
//! the current display's actual wallpaper, using the same precedence as Swift.
#![allow(unexpected_cfgs)]
use base64::Engine;
use objc::{class, msg_send, runtime::Object, sel, sel_impl, Encode, Encoding};
use plist::Value;
use serde_json::json;
use std::{ffi::CStr, path::PathBuf, sync::Mutex, time::SystemTime};
use tauri::Manager;

#[derive(Clone, Debug, PartialEq)]
enum Selection {
    Image(PathBuf),
    Aerial(String),
    Unavailable,
}
fn get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.as_dictionary()?.get(key)
}
fn file_url(value: Option<&Value>) -> Option<PathBuf> {
    let value = value?;
    let text = value
        .as_string()
        .or_else(|| get(value, "relative")?.as_string())?;
    url::Url::parse(text).ok()?.to_file_path().ok()
}
fn choice(value: Option<&Value>) -> Option<Selection> {
    let entry = value?;
    let section = if get(entry, "Type").and_then(Value::as_string) == Some("linked") {
        "Linked"
    } else {
        "Desktop"
    };
    let content = get(get(entry, section)?, "Content")?;
    let choices = get(content, "Choices")?.as_array()?;
    if choices.is_empty() {
        return None;
    }
    if choices.len() != 1 || get(content, "Shuffle").is_some_and(|v| v.as_string() != Some("$null"))
    {
        return Some(Selection::Unavailable);
    }
    let value = &choices[0];
    let config = get(value, "Configuration")
        .and_then(Value::as_data)
        .and_then(|data| Value::from_reader(std::io::Cursor::new(data)).ok());
    Some(match get(value, "Provider").and_then(Value::as_string) {
        Some("com.apple.wallpaper.choice.aerials") => config
            .as_ref()
            .and_then(|v| get(v, "assetID")?.as_string())
            .filter(|id| uuid::Uuid::parse_str(id).is_ok())
            .map(|id| Selection::Aerial(id.to_owned()))
            .unwrap_or(Selection::Unavailable),
        Some("com.apple.wallpaper.choice.image") => {
            file_url(config.as_ref().and_then(|v| get(v, "url")))
                .or_else(|| {
                    let files = get(value, "Files")?.as_array()?;
                    if files.len() == 1 {
                        file_url(files.first())
                    } else {
                        None
                    }
                })
                .map(Selection::Image)
                .unwrap_or(Selection::Unavailable)
        }
        _ => Selection::Unavailable,
    })
}
fn display<'a>(value: &'a Value, id: &str) -> Option<&'a Value> {
    get(value, "Displays")?
        .as_dictionary()?
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(id))
        .map(|(_, value)| value)
}
fn selection(store: &Value, id: &str, workspace: Option<PathBuf>) -> Selection {
    if let Some(value) = choice(get(store, "AllSpacesAndDisplays")) {
        return value;
    }
    if let Some(value) = choice(display(store, id)) {
        return value;
    }
    if let Some(path) =
        workspace.filter(|p| p.file_name().is_some_and(|n| n != "DefaultDesktop.heic"))
    {
        return Selection::Image(path);
    }
    if let Some(spaces) = get(store, "Spaces").and_then(Value::as_dictionary) {
        let choices: Vec<_> = spaces
            .values()
            .map(|space| {
                if let Some(entry) = display(space, id) {
                    choice(Some(entry))
                } else {
                    choice(get(space, "Default")).or_else(|| choice(Some(space)))
                }
            })
            .collect();
        if let Some(Some(first)) = choices.first() {
            if choices.iter().all(|c| c.as_ref() == Some(first)) {
                return first.clone();
            }
        }
    }
    Selection::Unavailable
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGDisplayCreateUUIDFromDisplayID(display: u32) -> *mut std::ffi::c_void;
    fn CGImageRelease(image: *mut std::ffi::c_void);
}
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFUUIDCreateString(
        allocator: *const std::ffi::c_void,
        uuid: *const std::ffi::c_void,
    ) -> *mut Object;
    fn CFRelease(value: *const std::ffi::c_void);
}
#[link(name = "AVFoundation", kind = "framework")]
extern "C" {}
#[repr(C)]
struct MediaTime {
    value: i64,
    timescale: i32,
    flags: u32,
    epoch: i64,
}
unsafe impl Encode for MediaTime {
    fn encode() -> Encoding {
        unsafe { Encoding::from_str("{CMTime=qiIq}") }
    }
}
unsafe fn ns(text: &str) -> *mut Object {
    let value: *mut Object = msg_send![class!(NSString), alloc];
    let value: *mut Object =
        msg_send![value, initWithBytes:text.as_ptr() length:text.len() encoding:4usize];
    msg_send![value, autorelease]
}
unsafe fn string(value: *mut Object) -> Option<String> {
    if value.is_null() {
        return None;
    }
    let bytes: *const std::ffi::c_char = msg_send![value, UTF8String];
    if bytes.is_null() {
        None
    } else {
        Some(CStr::from_ptr(bytes).to_string_lossy().into_owned())
    }
}
// Must be called on AppKit's main thread. Native objects never cross threads.
unsafe fn context(window: &tauri::WebviewWindow) -> Option<(String, Option<PathBuf>)> {
    let native = window.ns_window().ok()? as *mut Object;
    let mask: usize = msg_send![native, styleMask];
    if mask & (1 << 14) == 0 {
        return None;
    }
    let screen: *mut Object = msg_send![native, screen];
    if screen.is_null() {
        return None;
    }
    let device: *mut Object = msg_send![screen, deviceDescription];
    let number: *mut Object = msg_send![device, objectForKey:ns("NSScreenNumber")];
    let id: u32 = msg_send![number, unsignedIntValue];
    let uuid = CGDisplayCreateUUIDFromDisplayID(id);
    if uuid.is_null() {
        return None;
    }
    let uuid_string = CFUUIDCreateString(std::ptr::null(), uuid);
    let id = string(uuid_string).unwrap_or_default();
    CFRelease(uuid_string.cast());
    CFRelease(uuid);
    let workspace: *mut Object = msg_send![class!(NSWorkspace), sharedWorkspace];
    let url: *mut Object = msg_send![workspace, desktopImageURLForScreen:screen];
    let path: *mut Object = msg_send![url, path];
    Some((id, string(path).map(PathBuf::from)))
}
unsafe fn image(path: &std::path::Path, video: bool) -> Option<Vec<u8>> {
    let url: *mut Object = msg_send![class!(NSURL), fileURLWithPath:ns(&path.to_string_lossy())];
    let rep: *mut Object = if video {
        let asset: *mut Object =
            msg_send![class!(AVURLAsset), URLAssetWithURL:url options:std::ptr::null::<Object>()];
        let generator: *mut Object = msg_send![class!(AVAssetImageGenerator), alloc];
        let generator: *mut Object = msg_send![generator, initWithAsset:asset];
        let _: () = msg_send![generator, setAppliesPreferredTrackTransform:true];
        let time = MediaTime {
            value: 0,
            timescale: 1,
            flags: 1,
            epoch: 0,
        };
        let cg: *mut std::ffi::c_void = msg_send![generator, copyCGImageAtTime:time actualTime:std::ptr::null_mut::<MediaTime>() error:std::ptr::null_mut::<*mut Object>()];
        let _: () = msg_send![generator, release];
        if cg.is_null() {
            return None;
        }
        let rep: *mut Object = msg_send![class!(NSBitmapImageRep), alloc];
        let rep: *mut Object = msg_send![rep, initWithCGImage:cg];
        CGImageRelease(cg);
        let rep: *mut Object = msg_send![rep, autorelease];
        rep
    } else {
        let data: *mut Object = msg_send![class!(NSData), dataWithContentsOfURL:url];
        if data.is_null() {
            return None;
        }
        msg_send![class!(NSBitmapImageRep), imageRepWithData:data]
    };
    if rep.is_null() {
        return None;
    }
    let properties: *mut Object = msg_send![class!(NSDictionary), dictionary];
    let data: *mut Object = msg_send![rep, representationUsingType:4usize properties:properties];
    if data.is_null() {
        return None;
    }
    let length: usize = msg_send![data, length];
    let bytes: *const u8 = msg_send![data, bytes];
    if bytes.is_null() {
        return None;
    }
    Some(std::slice::from_raw_parts(bytes, length).to_vec())
}
struct Cached {
    path: PathBuf,
    modified: Option<SystemTime>,
    key: String,
    data: String,
}
static CACHE: Mutex<Option<Cached>> = Mutex::new(None);
pub fn background(
    app: &tauri::AppHandle,
    label: &str,
    previous_key: &str,
) -> crate::files::Result<serde_json::Value> {
    let window = app.get_webview_window(label).ok_or("Window unavailable")?;
    let (tx, rx) = std::sync::mpsc::channel();
    let target = window.clone();
    window
        .run_on_main_thread(move || unsafe {
            let _ = tx.send(context(&target));
        })
        .map_err(crate::files::err)?;
    let Some((id, workspace)) = rx.recv().map_err(crate::files::err)? else {
        return Ok(serde_json::Value::Null);
    };
    let home = dirs::home_dir().ok_or("Home folder unavailable")?;
    let store = Value::from_file(
        home.join("Library/Application Support/com.apple.wallpaper/Store/Index.plist"),
    )
    .unwrap_or(Value::Dictionary(Default::default()));
    let chosen = selection(&store, &id, workspace);
    let video = matches!(chosen, Selection::Aerial(_));
    let path = match chosen {
        Selection::Image(path) => path,
        Selection::Aerial(id) => {
            let candidates = [home.join(format!("Library/Application Support/com.apple.wallpaper/aerials/videos/{id}.mov")),
                PathBuf::from(format!("/Library/Application Support/com.apple.idleassetsd/Customer/4KSDR240FPS/{id}.mov"))];
            let Some(path) = candidates.into_iter().find(|p| p.is_file()) else {
                return Ok(serde_json::Value::Null);
            };
            path
        }
        Selection::Unavailable => return Ok(serde_json::Value::Null),
    };
    let modified = std::fs::metadata(&path)
        .ok()
        .and_then(|m| m.modified().ok());
    let mut cache = CACHE.lock().map_err(crate::files::err)?;
    if !cache
        .as_ref()
        .is_some_and(|c| c.path == path && c.modified == modified)
    {
        let bytes = unsafe {
            let pool: *mut Object = msg_send![class!(NSAutoreleasePool), new];
            let bytes = image(&path, video);
            let _: () = msg_send![pool, drain];
            bytes
        };
        let Some(bytes) = bytes else {
            *cache = None;
            return Ok(serde_json::Value::Null);
        };
        *cache = Some(Cached {
            path,
            modified,
            key: uuid::Uuid::new_v4().to_string(),
            data: format!(
                "data:image/png;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(bytes)
            ),
        });
    }
    let cached = cache.as_ref().unwrap();
    Ok(
        json!({"key":cached.key,"image":if previous_key == cached.key { None } else { Some(&cached.data) }}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn dict(entries: &[(&str, Value)]) -> Value {
        Value::Dictionary(
            entries
                .iter()
                .map(|(k, v)| ((*k).to_owned(), v.clone()))
                .collect(),
        )
    }
    fn entry(path: &str) -> Value {
        dict(&[(
            "Desktop",
            dict(&[(
                "Content",
                dict(&[(
                    "Choices",
                    Value::Array(vec![dict(&[
                        (
                            "Provider",
                            Value::String("com.apple.wallpaper.choice.image".into()),
                        ),
                        (
                            "Files",
                            Value::Array(vec![Value::String(format!("file://{path}"))]),
                        ),
                    ])]),
                )]),
            )]),
        )])
    }
    #[test]
    fn active_global_and_display_override_default_workspace() {
        let displays = dict(&[("DISPLAY", entry("/display.png"))]);
        let store = dict(&[
            ("AllSpacesAndDisplays", entry("/global.png")),
            ("Displays", displays.clone()),
            ("SystemDefault", entry("/wrong.png")),
        ]);
        assert_eq!(
            selection(&store, "display", Some("/wrong.png".into())),
            Selection::Image("/global.png".into())
        );
        assert_eq!(
            selection(&dict(&[("Displays", displays)]), "display", None),
            Selection::Image("/display.png".into())
        );
        assert_eq!(
            selection(
                &dict(&[("SystemDefault", entry("/wrong.png"))]),
                "display",
                Some("/DefaultDesktop.heic".into())
            ),
            Selection::Unavailable
        );
    }
    #[test]
    fn never_guess_different_spaces_or_shuffle() {
        let spaces = dict(&[
            ("a", dict(&[("Default", entry("/a.png"))])),
            ("b", dict(&[("Default", entry("/b.png"))])),
        ]);
        assert_eq!(
            selection(&dict(&[("Spaces", spaces)]), "display", None),
            Selection::Unavailable
        );
        assert!(file_url(Some(&Value::String("https://example.org/image.png".into()))).is_none());
        let mut shuffled = entry("/a.png");
        shuffled
            .as_dictionary_mut()
            .unwrap()
            .get_mut("Desktop")
            .unwrap()
            .as_dictionary_mut()
            .unwrap()
            .get_mut("Content")
            .unwrap()
            .as_dictionary_mut()
            .unwrap()
            .insert("Shuffle".into(), Value::Boolean(true));
        assert_eq!(
            selection(
                &dict(&[("AllSpacesAndDisplays", shuffled)]),
                "display",
                Some("/fallback.png".into())
            ),
            Selection::Unavailable
        );
    }
}
