// Capture the event's pasteboard before WebKit consumes it. Keep HTML drag/drop
// enabled so Source tabs and artwork continue to use their existing handlers.
#![allow(unexpected_cfgs)]
use crate::files::Result;
use serde_json::Value;

#[cfg(target_os = "macos")]
mod mac {
    use super::*;
    use objc::{
        msg_send,
        runtime::{
            class_addMethod, method_setImplementation, Class, Imp, Method, Object, Sel, BOOL, NO,
            YES,
        },
        sel, sel_impl,
    };
    use std::{
        collections::HashMap,
        path::PathBuf,
        sync::{Mutex, OnceLock},
        time::{Duration, Instant},
    };

    extern "C" {
        fn method_getTypeEncoding(method: *const Method) -> *const std::ffi::c_char;
    }
    struct Drop {
        paths: Vec<PathBuf>,
        time: Instant,
    }
    static DROP: Mutex<Option<Drop>> = Mutex::new(None);
    #[derive(Clone)]
    struct Target {
        label: String,
        app: Option<tauri::AppHandle>,
    }
    static VIEWS: OnceLock<Mutex<HashMap<usize, Target>>> = OnceLock::new();
    fn views() -> &'static Mutex<HashMap<usize, Target>> {
        VIEWS.get_or_init(Mutex::default)
    }
    static ENTERED: OnceLock<Imp> = OnceLock::new();
    static UPDATED: OnceLock<Imp> = OnceLock::new();
    static ORIGINAL: OnceLock<Imp> = OnceLock::new();
    static PREPARED: OnceLock<Imp> = OnceLock::new();
    #[cfg(test)]
    unsafe fn ns(text: &str) -> *mut Object {
        let value: *mut Object = msg_send![objc::class!(NSString), alloc];
        let value: *mut Object =
            msg_send![value, initWithBytes:text.as_ptr() length:text.len() encoding:4usize];
        let _: *mut Object = msg_send![value, autorelease];
        value
    }
    unsafe fn read_board(board: *mut Object) -> Vec<PathBuf> {
        crate::native_import::mac::file_paths(board)
    }
    fn target(this: &Object) -> Option<Target> {
        views()
            .lock()
            .ok()?
            .get(&(this as *const Object as usize))
            .cloned()
    }
    unsafe fn external(info: *mut Object) -> bool {
        let supports: BOOL = msg_send![info, respondsToSelector:sel!(draggingSource)];
        if supports == NO {
            return true;
        }
        let source: *mut Object = msg_send![info, draggingSource];
        source.is_null()
    }
    unsafe fn explicit_svg(info: *mut Object) -> bool {
        if !external(info) {
            return false;
        }
        let board: *mut Object = msg_send![info, draggingPasteboard];
        crate::native_import::mac::types(board).iter().any(|kind| {
            let lower = kind.to_ascii_lowercase();
            lower.contains("svg") || lower == "com.microsoft.art--gvml-clipformat"
        })
    }
    extern "C" fn entered(this: &Object, selector: Sel, info: *mut Object) -> usize {
        unsafe {
            if target(this).is_some() && explicit_svg(info) {
                return 1;
            } // NSDragOperationCopy
            let original: unsafe extern "C" fn(&Object, Sel, *mut Object) -> usize =
                std::mem::transmute(*ENTERED.get().unwrap());
            original(this, selector, info)
        }
    }
    extern "C" fn updated(this: &Object, selector: Sel, info: *mut Object) -> usize {
        unsafe {
            if target(this).is_some() && explicit_svg(info) {
                return 1;
            }
            let original: unsafe extern "C" fn(&Object, Sel, *mut Object) -> usize =
                std::mem::transmute(*UPDATED.get().unwrap());
            original(this, selector, info)
        }
    }
    extern "C" fn prepare(this: &Object, selector: Sel, info: *mut Object) -> BOOL {
        unsafe {
            if target(this).is_some() && explicit_svg(info) {
                return YES;
            }
            let original: unsafe extern "C" fn(&Object, Sel, *mut Object) -> BOOL =
                std::mem::transmute(*PREPARED.get().unwrap());
            original(this, selector, info)
        }
    }
    extern "C" fn perform(this: &Object, selector: Sel, info: *mut Object) -> BOOL {
        unsafe {
            if let Some(target) = target(this) {
                let board: *mut Object = msg_send![info, draggingPasteboard];
                if std::env::var("MATHLIDELUX_DROP_DIAGNOSTICS").as_deref() == Ok("1") {
                    let kinds = crate::native_import::mac::types(board);
                    eprintln!(
                        "Mathlidelux drop: {} types {:?}; internal={}",
                        kinds.len(),
                        kinds,
                        !external(info)
                    );
                }
                // Internal cards/tabs retain HTML drag semantics. The event's
                // actual pasteboard, never the general clipboard, is authoritative.
                if external(info) {
                    if let Some(app) = target.app {
                        match crate::native_import::mac::svg(board) {
                            Ok(sources) if !sources.is_empty() => {
                                let destination = if target.label == "main" {
                                    "workspace".to_owned()
                                } else {
                                    target.label.clone()
                                };
                                tauri::async_runtime::spawn_blocking(move || {
                                    if let Err(error) =
                                        crate::resident::import_svg_to(&app, &sources, &destination)
                                    {
                                        crate::resident::report(&app, error);
                                    }
                                });
                                // WebKit must not import the same external SVG a second time.
                                return YES;
                            }
                            Err(error) if explicit_svg(info) => {
                                crate::resident::report(&app, error);
                                return YES;
                            }
                            _ => {}
                        }
                    }
                    if target.label == "main" {
                        if let Ok(mut captured) = DROP.lock() {
                            *captured = Some(Drop {
                                paths: read_board(board),
                                time: Instant::now(),
                            });
                        }
                    }
                }
            }
            // Preserve Wry/WebKit's implementation and the AppKit view identity.
            let original: unsafe extern "C" fn(&Object, Sel, *mut Object) -> BOOL =
                std::mem::transmute(*ORIGINAL.get().unwrap());
            original(this, selector, info)
        }
    }
    pub fn install(window: &tauri::WebviewWindow) -> tauri::Result<()> {
        use tauri::Manager;
        let target = Target {
            label: window.label().to_owned(),
            app: Some(window.app_handle().clone()),
        };
        window.with_webview(move |webview| unsafe {
            let view = webview.inner().cast::<Object>();
            install_view(view, target);
            // Add, rather than replace, WebKit's existing HTML and file types.
            let current: *mut Object = msg_send![view, registeredDraggedTypes];
            let types: *mut Object = if current.is_null() {
                msg_send![objc::class!(NSMutableArray), new]
            } else {
                msg_send![current, mutableCopy]
            };
            for kind in [
                "public.svg-image",
                "image/svg+xml",
                "com.microsoft.image-svg-xml",
                "com.microsoft.Art--GVML-ClipFormat",
            ] {
                let _: () = msg_send![types, addObject:crate::native_import::mac::ns(kind)];
            }
            let _: () = msg_send![view, registerForDraggedTypes:types];
            let _: () = msg_send![types, release];
        })
    }
    unsafe fn install_view(view: *mut Object, target: Target) {
        let mut class = Some((*view).class());
        let mut wry = None;
        while let Some(candidate) = class {
            if candidate.name().starts_with("wry::") && candidate.name().contains("::WryWebView") {
                wry = Some(candidate);
                break;
            }
            class = candidate.superclass();
        }
        let Some(wry) = wry else {
            eprintln!(
                "Native import hook unavailable for {}",
                (*view).class().name()
            );
            return;
        };
        let Some(method) = wry.instance_method(sel!(performDragOperation:)) else {
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
        for (selector, saved, replacement) in [
            (
                sel!(draggingEntered:),
                &ENTERED,
                entered as extern "C" fn(&Object, Sel, *mut Object) -> usize,
            ),
            (sel!(draggingUpdated:), &UPDATED, updated),
        ] {
            if let Some(method) = wry.instance_method(selector) {
                saved.get_or_init(|| method_setImplementation(method as *const Method as *mut Method,
                    std::mem::transmute::<extern "C" fn(&Object, Sel, *mut Object) -> usize, Imp>(replacement)));
            }
        }
        if let Some(method) = wry.instance_method(sel!(prepareForDragOperation:)) {
            PREPARED.get_or_init(|| {
                let original = method.implementation();
                let replacement = std::mem::transmute::<
                    extern "C" fn(&Object, Sel, *mut Object) -> BOOL,
                    Imp,
                >(prepare);
                // prepare is normally inherited from WKWebView: override on Wry
                // instead of changing WebKit's implementation for every app view.
                if class_addMethod(
                    wry as *const Class as *mut Class,
                    sel!(prepareForDragOperation:),
                    replacement,
                    method_getTypeEncoding(method as *const Method as *mut Method),
                ) == NO
                {
                    method_setImplementation(method as *const Method as *mut Method, replacement);
                }
                original
            });
        }
        views().lock().unwrap().insert(view as usize, target);
    }
    pub fn clear(label: &str) {
        views()
            .lock()
            .unwrap()
            .retain(|_, target| target.label != label);
        if label == "main" {
            if let Ok(mut captured) = DROP.lock() {
                *captured = None;
            }
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
        use objc::class;
        use serde_json::json;
        unsafe fn test_board() -> *mut Object {
            // Keep parallel AppKit tests on explicitly different pasteboards.
            let name = format!("Mathlidelux-drop-test-{}", uuid::Uuid::new_v4());
            msg_send![class!(NSPasteboard), pasteboardWithName:ns(&name)]
        }
        #[test]
        fn hooks_versioned_wry_class_without_changing_its_identity_and_forwards_html_drop() {
            use objc::{
                declare::ClassDecl,
                runtime::{Class, YES},
            };
            extern "C" fn original(_: &Object, _: Sel, _: *mut Object) -> BOOL {
                YES
            }
            extern "C" fn rejected(_: &Object, _: Sel, _: *mut Object) -> BOOL {
                NO
            }
            extern "C" fn no_operation(_: &Object, _: Sel, _: *mut Object) -> usize {
                0
            }
            extern "C" fn source(this: &Object, _: Sel) -> *mut Object {
                unsafe { *this.get_ivar::<*mut Object>("source") }
            }
            extern "C" fn board(this: &Object, _: Sel) -> *mut Object {
                unsafe { *this.get_ivar::<*mut Object>("board") }
            }
            unsafe {
                let pool: *mut Object = msg_send![class!(NSAutoreleasePool), new];
                let mut parent =
                    ClassDecl::new("MathlideluxTestWebKitParent", class!(NSObject)).unwrap();
                parent.add_method(
                    sel!(prepareForDragOperation:),
                    rejected as extern "C" fn(&Object, Sel, *mut Object) -> BOOL,
                );
                let parent = parent.register();
                let mut declaration =
                    ClassDecl::new("wry::test::WryWebView0.57.0", parent).unwrap();
                for selector in [sel!(draggingEntered:), sel!(draggingUpdated:)] {
                    declaration.add_method(
                        selector,
                        no_operation as extern "C" fn(&Object, Sel, *mut Object) -> usize,
                    );
                }
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
                declaration.add_ivar::<*mut Object>("source");
                declaration.add_method(
                    sel!(draggingSource),
                    source as extern "C" fn(&Object, Sel) -> *mut Object,
                );
                declaration.add_method(
                    sel!(draggingPasteboard),
                    board as extern "C" fn(&Object, Sel) -> *mut Object,
                );
                let info_class = declaration.register();
                let info: *mut Object = msg_send![info_class, new];
                let pasteboard = test_board();
                let url: *mut Object =
                    msg_send![class!(NSURL), fileURLWithPath:ns("/tmp/Test folder")];
                let urls: *mut Object = msg_send![class!(NSArray), arrayWithObject:url];
                let _: BOOL = msg_send![pasteboard, writeObjects:urls];
                (*info).set_ivar("board", pasteboard);
                install_view(
                    view,
                    Target {
                        label: "main".into(),
                        app: None,
                    },
                );
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
                let _: i64 = msg_send![pasteboard, clearContents];
                let bytes = b"<svg></svg>";
                let data: *mut Object =
                    msg_send![class!(NSData), dataWithBytes:bytes.as_ptr() length:bytes.len()];
                let _: BOOL =
                    msg_send![pasteboard, setData:data forType:ns("com.microsoft.image-svg-xml")];
                let accepted: usize = msg_send![view, draggingEntered:info];
                let prepared: BOOL = msg_send![view, prepareForDragOperation:info];
                assert_eq!((accepted, prepared), (1, YES));
                let untouched: BOOL = msg_send![other, prepareForDragOperation:info];
                assert_eq!(untouched, NO);
                (*info).set_ivar("source", view);
                let accepted: usize = msg_send![view, draggingEntered:info];
                let prepared: BOOL = msg_send![view, prepareForDragOperation:info];
                assert_eq!((accepted, prepared), (0, NO));
                clear("main");
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
                let board = test_board();
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
