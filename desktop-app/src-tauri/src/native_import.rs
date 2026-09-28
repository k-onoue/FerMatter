//! Read the clipboard only after an explicit Edit from Clipboard command.
#![allow(unexpected_cfgs)]
use crate::files::*;

#[cfg(target_os = "macos")]
pub fn clipboard() -> Result<Vec<String>> {
    use objc::{class, msg_send, runtime::Object, sel, sel_impl};
    use std::{ffi::CStr, path::Path};
    unsafe fn ns(text: &str) -> *mut Object {
        let value: *mut Object = msg_send![class!(NSString), alloc];
        let value: *mut Object =
            msg_send![value, initWithBytes:text.as_ptr() length:text.len() encoding:4usize];
        let _: *mut Object = msg_send![value, autorelease];
        value
    }
    unsafe fn read(board: *mut Object) -> Result<Vec<String>> {
        for kind in ["public.svg-image", "image/svg+xml"] {
            let data: *mut Object = msg_send![board, dataForType:ns(kind)];
            if !data.is_null() {
                let len: usize = msg_send![data, length];
                if len as u64 > crate::roundtrip::SVG_LIMIT {
                    return Err("SVG exceeds 16 MB.".into());
                }
                if len == 0 {
                    continue;
                }
                let bytes: *const u8 = msg_send![data, bytes];
                if !bytes.is_null() {
                    return Ok(vec![String::from_utf8(
                        std::slice::from_raw_parts(bytes, len).to_vec(),
                    )
                    .map_err(err)?]);
                }
            }
        }
        let classes: *mut Object = msg_send![class!(NSArray), arrayWithObject:class!(NSURL)];
        let urls: *mut Object =
            msg_send![board, readObjectsForClasses:classes options:std::ptr::null::<Object>()];
        let count: usize = msg_send![urls, count];
        if count > 16 {
            return Err("Open up to 16 SVG files at once.".into());
        }
        let mut result = vec![];
        for index in 0..count {
            let url: *mut Object = msg_send![urls, objectAtIndex:index];
            let file: bool = msg_send![url, isFileURL];
            if !file {
                continue;
            }
            let path: *mut Object = msg_send![url, path];
            let bytes: *const std::ffi::c_char = msg_send![path, UTF8String];
            if !bytes.is_null() {
                result.push(crate::roundtrip::read_svg(Path::new(
                    &*CStr::from_ptr(bytes).to_string_lossy(),
                ))?);
            }
        }
        if !result.is_empty() {
            return Ok(result);
        }
        Err("The clipboard has no editable SVG. Copy the original SVG file, or use Instant history.".into())
    }
    unsafe {
        let pool: *mut Object = msg_send![class!(NSAutoreleasePool), new];
        let board: *mut Object = msg_send![class!(NSPasteboard), generalPasteboard];
        let result = read(board);
        let _: () = msg_send![pool, drain];
        result
    }
}
#[cfg(not(target_os = "macos"))]
pub fn clipboard() -> Result<Vec<String>> {
    Err("SVG clipboard import is currently supported on macOS.".into())
}
