// objc 0.2 uses a legacy cargo-clippy cfg in its message macros.
#![allow(unexpected_cfgs)]
use crate::files::*;
use std::path::PathBuf;
#[cfg(target_os = "macos")]
pub fn clipboard(paths: &[PathBuf]) -> Result<()> {
    use objc::{class, msg_send, runtime::Object, sel, sel_impl};
    unsafe fn ns(text: &str) -> *mut Object {
        let value: *mut Object = msg_send![class!(NSString), alloc];
        let value: *mut Object =
            msg_send![value,initWithBytes:text.as_ptr() length:text.len() encoding:4usize];
        let _: *mut Object = msg_send![value, autorelease];
        value
    }
    let bytes = if paths.len() == 1 {
        Some(std::fs::read(&paths[0]).map_err(err)?)
    } else {
        None
    };
    unsafe {
        let pool: *mut Object = msg_send![class!(NSAutoreleasePool), new];
        let array: *mut Object = msg_send![class!(NSMutableArray), array];
        for path in paths {
            let item: *mut Object = msg_send![class!(NSPasteboardItem), new];
            let url: *mut Object =
                msg_send![class!(NSURL),fileURLWithPath:ns(&path.to_string_lossy())];
            let absolute: *mut Object = msg_send![url, absoluteString];
            let _: bool = msg_send![item,setString:absolute forType:ns("public.file-url")];
            if paths.len() == 1 {
                let bytes = bytes.as_ref().unwrap();
                let data: *mut Object =
                    msg_send![class!(NSData),dataWithBytes:bytes.as_ptr() length:bytes.len()];
                let kind = match path.extension().and_then(|s| s.to_str()).unwrap_or("") {
                    "svg" => "public.svg-image",
                    "pdf" => "com.adobe.pdf",
                    "png" => "public.png",
                    "jpg" | "jpeg" => "public.jpeg",
                    _ => "public.data",
                };
                let _: bool = msg_send![item,setData:data forType:ns(kind)];
            }
            let _: () = msg_send![array,addObject:item];
            let _: () = msg_send![item, release];
        }
        let board: *mut Object = msg_send![class!(NSPasteboard), generalPasteboard];
        let _: isize = msg_send![board, clearContents];
        let success: bool = msg_send![board,writeObjects:array];
        let _: () = msg_send![pool, drain];
        if success {
            Ok(())
        } else {
            Err("Unable to copy images.".into())
        }
    }
}
#[cfg(not(target_os = "macos"))]
pub fn clipboard(_: &[PathBuf]) -> Result<()> {
    Err("Native image clipboard is currently supported on macOS.".into())
}
