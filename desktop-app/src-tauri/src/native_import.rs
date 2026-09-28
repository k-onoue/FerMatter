//! Explicit clipboard commands and actual NSDraggingInfo pasteboards share decoding.
#![allow(unexpected_cfgs)]
use crate::files::*;
#[cfg(target_os = "macos")]
pub mod mac {
    use super::*;
    use objc::{
        class, msg_send,
        runtime::{Object, BOOL, NO},
        sel, sel_impl,
    };
    use std::{ffi::CStr, path::PathBuf};
    pub unsafe fn ns(text: &str) -> *mut Object {
        let value: *mut Object = msg_send![class!(NSString), alloc];
        let value: *mut Object =
            msg_send![value, initWithBytes:text.as_ptr() length:text.len() encoding:4usize];
        let _: *mut Object = msg_send![value, autorelease];
        value
    }
    unsafe fn text(value: *mut Object) -> Option<String> {
        if value.is_null() {
            return None;
        }
        let bytes: *const std::ffi::c_char = msg_send![value, UTF8String];
        (!bytes.is_null()).then(|| CStr::from_ptr(bytes).to_string_lossy().into_owned())
    }
    pub unsafe fn types(board: *mut Object) -> Vec<String> {
        if board.is_null() {
            return vec![];
        }
        let types: *mut Object = msg_send![board, types];
        let count: usize = msg_send![types, count];
        (0..count.min(128))
            .filter_map(|index| {
                let kind: *mut Object = msg_send![types, objectAtIndex:index];
                text(kind)
            })
            .collect()
    }
    pub unsafe fn file_paths(board: *mut Object) -> Vec<PathBuf> {
        if board.is_null() {
            return vec![];
        }
        let classes: *mut Object = msg_send![class!(NSArray), arrayWithObject:class!(NSURL)];
        let urls: *mut Object =
            msg_send![board, readObjectsForClasses:classes options:std::ptr::null::<Object>()];
        let count: usize = msg_send![urls, count];
        // Finder project batches have a separate 1000-item import limit.
        (0..count.min(1001))
            .filter_map(|index| {
                let url: *mut Object = msg_send![urls, objectAtIndex:index];
                let file: BOOL = msg_send![url, isFileURL];
                if file == NO {
                    return None;
                }
                let path: *mut Object = msg_send![url, path];
                text(path).map(PathBuf::from)
            })
            .collect()
    }
    pub unsafe fn svg(board: *mut Object) -> Result<Vec<String>> {
        let mut kinds = types(board);
        // Prefer original SVG over HTML previews or Office bitmap fallbacks.
        kinds.sort_by_key(|kind| {
            if kind.to_ascii_lowercase().contains("svg") {
                0
            } else {
                1
            }
        });
        let mut inspected = 0;
        for kind in kinds {
            let lower = kind.to_ascii_lowercase();
            if ![
                "svg",
                "html",
                "text",
                "string",
                "microsoft",
                "office",
                "powerpoint",
            ]
            .iter()
            .any(|name| lower.contains(name))
            {
                continue;
            }
            let data: *mut Object = msg_send![board, dataForType:ns(&kind)];
            if data.is_null() {
                continue;
            }
            let length: usize = msg_send![data, length];
            inspected += length;
            if inspected > crate::svg_payload::BATCH_LIMIT {
                return Err("Dropped data exceeds 64 MB.".into());
            }
            if length == 0 {
                continue;
            }
            let bytes: *const u8 = msg_send![data, bytes];
            if bytes.is_null() {
                continue;
            }
            let found =
                crate::svg_payload::decode(&kind, std::slice::from_raw_parts(bytes, length))?;
            if !found.is_empty() {
                return Ok(found);
            }
        }
        let paths = file_paths(board);
        if paths.is_empty()
            || paths.iter().any(|path| {
                !path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("svg"))
            })
        {
            return Ok(vec![]);
        }
        if paths.len() > 16 {
            return Err("Open up to 16 SVG files at once.".into());
        }
        let mut output = Vec::new();
        let mut total = 0;
        for path in paths {
            let source = crate::roundtrip::read_svg(&path)?;
            total += source.len();
            if total > crate::svg_payload::BATCH_LIMIT {
                return Err("SVG selection exceeds 64 MB.".into());
            }
            output.push(source);
        }
        Ok(output)
    }
}
#[cfg(target_os = "macos")]
pub fn clipboard() -> Result<Vec<String>> {
    use objc::{class, msg_send, runtime::Object, sel, sel_impl};
    unsafe {
        let pool: *mut Object = msg_send![class!(NSAutoreleasePool), new];
        let board: *mut Object = msg_send![class!(NSPasteboard), generalPasteboard];
        let result = mac::svg(board).and_then(|sources| {
            if sources.is_empty() { Err("The clipboard has no editable SVG. Copy the original SVG file, or use Instant history.".into()) } else { Ok(sources) }
        });
        let _: () = msg_send![pool, drain];
        result
    }
}
#[cfg(not(target_os = "macos"))]
pub fn clipboard() -> Result<Vec<String>> {
    Err("SVG clipboard import is currently supported on macOS.".into())
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use objc::{
        class, msg_send,
        runtime::{Object, BOOL},
        sel, sel_impl,
    };
    #[test]
    fn powerpoint_svg_representation_wins_over_private_office_formats() {
        unsafe {
            let pool: *mut Object = msg_send![class!(NSAutoreleasePool), new];
            let board: *mut Object = msg_send![class!(NSPasteboard), pasteboardWithName:mac::ns(&format!("Mathlidelux-SVG-test-{}",uuid::Uuid::new_v4()))];
            let _: i64 = msg_send![board, clearContents];
            for (kind, bytes) in [
                ("com.microsoft.Art--GVML-ClipFormat", b"NOT XML".as_slice()),
                (
                    "com.microsoft.image-svg-xml",
                    b"<svg><metadata>preserved recipe</metadata></svg>".as_slice(),
                ),
            ] {
                let data: *mut Object =
                    msg_send![class!(NSData), dataWithBytes:bytes.as_ptr() length:bytes.len()];
                let _: BOOL = msg_send![board, setData:data forType:mac::ns(kind)];
            }
            assert_eq!(
                mac::svg(board).unwrap(),
                vec!["<svg><metadata>preserved recipe</metadata></svg>"]
            );
            let _: () = msg_send![board, releaseGlobally];
            let _: () = msg_send![pool, drain];
        }
    }
}
