// Capture the event's pasteboard before WebKit consumes it. Keep HTML drag/drop
// enabled so Source tabs and artwork continue to use their existing handlers.
#![allow(unexpected_cfgs)]
use crate::files::Result;
use serde_json::Value;

#[cfg(target_os = "macos")]
mod mac {
    use super::*;
    use objc::{
        class, msg_send,
        runtime::{method_setImplementation, Imp, Method, Object, Sel, BOOL, NO},
        sel, sel_impl,
    };
    use std::{
        ffi::CStr,
        path::PathBuf,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Mutex, OnceLock,
        },
        time::{Duration, Instant},
    };

    struct Drop {
        paths: Vec<PathBuf>,
        time: Instant,
    }
    static DROP: Mutex<Option<Drop>> = Mutex::new(None);
    static MAIN_VIEW: AtomicUsize = AtomicUsize::new(0);
    static ORIGINAL: OnceLock<Imp> = OnceLock::new();
    #[cfg(test)]
    unsafe fn ns(text: &str) -> *mut Object {
        let value: *mut Object = msg_send![class!(NSString), alloc];
        let value: *mut Object =
            msg_send![value, initWithBytes:text.as_ptr() length:text.len() encoding:4usize];
        let _: *mut Object = msg_send![value, autorelease];
        value
    }
    unsafe fn read_board(board: *mut Object) -> Vec<PathBuf> {
        if board.is_null() {
            return vec![];
        }
        let classes: *mut Object = msg_send![class!(NSArray), arrayWithObject:class!(NSURL)];
        let urls: *mut Object =
            msg_send![board, readObjectsForClasses:classes options:std::ptr::null::<Object>()];
        let count: usize = msg_send![urls, count];
        let mut paths = Vec::new();
        for index in 0..count {
            let url: *mut Object = msg_send![urls, objectAtIndex:index];
            let file: BOOL = msg_send![url, isFileURL];
            if file == NO {
                continue;
            }
            let path: *mut Object = msg_send![url, path];
            let bytes: *const std::ffi::c_char = msg_send![path, UTF8String];
            if !bytes.is_null() {
                paths.push(PathBuf::from(
                    CStr::from_ptr(bytes).to_string_lossy().into_owned(),
                ));
            }
        }
        paths
    }
    extern "C" fn perform(this: &Object, selector: Sel, info: *mut Object) -> BOOL {
        unsafe {
            if this as *const Object as usize == MAIN_VIEW.load(Ordering::Relaxed) {
                let board: *mut Object = msg_send![info, draggingPasteboard];
                if let Ok(mut captured) = DROP.lock() {
                    *captured = Some(Drop {
                        paths: read_board(board),
                        time: Instant::now(),
                    });
                }
            }
            // Forward directly to Wry's original implementation. Changing the
            // view's class after creation breaks AppKit's safe-area cache on macOS 26.
            let original: unsafe extern "C" fn(&Object, Sel, *mut Object) -> BOOL =
                std::mem::transmute(*ORIGINAL.get().unwrap());
            original(this, selector, info)
        }
    }
    pub fn install(window: &tauri::WebviewWindow) -> tauri::Result<()> {
        window.with_webview(|webview| unsafe {
            let view = webview.inner().cast::<Object>();
            install_view(view);
        })
    }
    unsafe fn install_view(view: *mut Object) {
        // Only hook the app-owned Wry class, never WKWebView or NSView.
        let mut class = Some((*view).class());
        let mut wry = None;
        while let Some(candidate) = class {
            if candidate.name().starts_with("wry::") && candidate.name().contains("::WryWebView") {
                wry = Some(candidate);
                break;
            }
            class = candidate.superclass();
        }
        let Some(method) = wry.and_then(|class| class.instance_method(sel!(performDragOperation:)))
        else {
            eprintln!(
                "Finder import hook unavailable for {}",
                (*view).class().name()
            );
            return;
        };
        ORIGINAL.get_or_init(|| {
            method_setImplementation(
                method as *const Method as *mut Method,
                std::mem::transmute::<extern "C" fn(&Object, Sel, *mut Object) -> BOOL, Imp>(
                    perform,
                ),
            )
        });
        MAIN_VIEW.store(view as usize, Ordering::Relaxed);
    }
    pub fn clear() {
        MAIN_VIEW.store(0, Ordering::Relaxed);
        if let Ok(mut captured) = DROP.lock() {
            *captured = None;
        }
    }
    fn consume(captured: &mut Option<Drop>, names: &Value) -> Result<Value> {
        let dropped = captured.take().ok_or(
            "No native file drop was received. Please drag the folder or ZIP from Finder again.",
        )?;
        let mut expected: Vec<&str> = names
            .as_array()
            .ok_or("Missing dropped files")?
            .iter()
            .map(|name| name.as_str().ok_or("Invalid dropped filename"))
            .collect::<std::result::Result<_, _>>()?;
        let mut actual: Vec<_> = dropped
            .paths
            .iter()
            .map(|path| path.file_name().unwrap_or_default().to_string_lossy())
            .collect();
        expected.sort();
        actual.sort();
        if expected.is_empty()
            || expected != actual
            || dropped.time.elapsed() > Duration::from_secs(10)
        {
            return Err(
                format!("Could not match the dropped files (Finder: {}, browser: {}). Please drag them from Finder again.", actual.len(), expected.len()),
            );
        }
        Ok(serde_json::json!(dropped.paths))
    }
    pub fn paths(names: &Value) -> Result<Value> {
        consume(
            &mut *DROP.lock().map_err(|_| "Drop reader unavailable")?,
            names,
        )
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        use serde_json::json;
        #[test]
        fn hooks_versioned_wry_class_without_changing_its_identity_and_forwards_html_drop() {
            use objc::{
                declare::ClassDecl,
                runtime::{Class, YES},
            };
            extern "C" fn original(_: &Object, _: Sel, _: *mut Object) -> BOOL {
                YES
            }
            extern "C" fn board(this: &Object, _: Sel) -> *mut Object {
                unsafe { *this.get_ivar::<*mut Object>("board") }
            }
            unsafe {
                let pool: *mut Object = msg_send![class!(NSAutoreleasePool), new];
                let mut declaration =
                    ClassDecl::new("wry::test::WryWebView0.57.0", class!(NSObject)).unwrap();
                declaration.add_method(
                    sel!(performDragOperation:),
                    original as extern "C" fn(&Object, Sel, *mut Object) -> BOOL,
                );
                let class: &Class = declaration.register();
                let view: *mut Object = msg_send![class, new];
                let other: *mut Object = msg_send![class, new];
                let mut declaration =
                    ClassDecl::new("MathlideluxTestDragInfo", class!(NSObject)).unwrap();
                declaration.add_ivar::<*mut Object>("board");
                declaration.add_method(
                    sel!(draggingPasteboard),
                    board as extern "C" fn(&Object, Sel) -> *mut Object,
                );
                let info_class = declaration.register();
                let info: *mut Object = msg_send![info_class, new];
                let pasteboard: *mut Object =
                    msg_send![class!(NSPasteboard), pasteboardWithUniqueName];
                let url: *mut Object =
                    msg_send![class!(NSURL), fileURLWithPath:ns("/tmp/Test folder")];
                let urls: *mut Object = msg_send![class!(NSArray), arrayWithObject:url];
                let _: BOOL = msg_send![pasteboard, writeObjects:urls];
                (*info).set_ivar("board", pasteboard);
                install_view(view);
                assert_eq!((*view).class(), class);
                let forwarded: BOOL = msg_send![view, performDragOperation:info];
                assert_eq!(forwarded, YES);
                assert_eq!(
                    paths(&json!(["Test folder"])).unwrap(),
                    json!(["/tmp/Test folder"])
                );
                let forwarded: BOOL = msg_send![other, performDragOperation:info];
                assert_eq!(forwarded, YES);
                assert!(paths(&json!(["Test folder"])).is_err());
                clear();
                let _: () = msg_send![info, release];
                let _: () = msg_send![view, release];
                let _: () = msg_send![other, release];
                let _: () = msg_send![pasteboard, releaseGlobally];
                let _: () = msg_send![pool, drain];
            }
        }
        #[test]
        fn paths_are_scoped_to_one_recent_matching_drop() {
            let paths = vec![
                PathBuf::from("/tmp/数式.zip"),
                PathBuf::from("/tmp/Diagram folder"),
            ];
            let drop = || {
                Some(Drop {
                    paths: paths.clone(),
                    time: Instant::now(),
                })
            };
            let names = json!(["Diagram folder", "数式.zip"]);
            let mut captured = drop();
            assert_eq!(consume(&mut captured, &names).unwrap(), json!(paths));
            assert!(consume(&mut captured, &names).is_err());
            assert!(consume(&mut drop(), &json!(["unrelated.zip"])).is_err());
            assert!(consume(&mut drop(), &json!([])).is_err());
            let mut stale = Some(Drop {
                paths,
                time: Instant::now() - Duration::from_secs(11),
            });
            assert!(consume(&mut stale, &names).is_err());
        }
        #[test]
        fn reads_file_urls_from_the_events_pasteboard() {
            unsafe {
                let pool: *mut Object = msg_send![class!(NSAutoreleasePool), new];
                let board: *mut Object = msg_send![class!(NSPasteboard), pasteboardWithUniqueName];
                let urls: *mut Object = msg_send![class!(NSMutableArray), array];
                for path in ["/tmp/数式.zip", "/tmp/Diagram folder"] {
                    let url: *mut Object = msg_send![class!(NSURL), fileURLWithPath:ns(path)];
                    let _: () = msg_send![urls, addObject:url];
                }
                let _: BOOL = msg_send![board, writeObjects:urls];
                assert_eq!(
                    read_board(board),
                    vec![
                        PathBuf::from("/tmp/数式.zip"),
                        PathBuf::from("/tmp/Diagram folder")
                    ]
                );
                let _: () = msg_send![board, releaseGlobally];
                let _: () = msg_send![pool, drain];
            }
        }
    }
}
#[cfg(target_os = "macos")]
pub use mac::{clear, install, paths};
#[cfg(not(target_os = "macos"))]
pub fn paths(_: &Value) -> Result<Value> {
    Err("Finder imports are currently supported on macOS.".into())
}
