// objc 0.2 uses a legacy cargo-clippy cfg in its message macros.
#![allow(unexpected_cfgs)]
use crate::files::*;
use std::path::PathBuf;
#[cfg(target_os = "macos")]
pub fn clipboard(paths: &[PathBuf]) -> Result<()> {
    clipboard_to(paths, None)
}

#[cfg(target_os = "macos")]
fn clipboard_to(paths: &[PathBuf], name: Option<&str>) -> Result<()> {
    use objc::{class, msg_send, runtime::Object, sel, sel_impl};
    unsafe fn ns(text: &str) -> *mut Object {
        let value: *mut Object = msg_send![class!(NSString), alloc];
        let value: *mut Object =
            msg_send![value,initWithBytes:text.as_ptr() length:text.len() encoding:4usize];
        let _: *mut Object = msg_send![value, autorelease];
        value
    }
    // Validate the whole selection before replacing the user's clipboard.
    if paths.is_empty() {
        return Err("No images selected.".into());
    }
    let bytes = paths
        .iter()
        .map(std::fs::read)
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(err)?;
    unsafe {
        let pool: *mut Object = msg_send![class!(NSAutoreleasePool), new];
        let array: *mut Object = msg_send![class!(NSMutableArray), array];
        let filenames: *mut Object = msg_send![class!(NSMutableArray), array];
        for path in paths {
            let item: *mut Object = msg_send![class!(NSPasteboardItem), new];
            let url: *mut Object =
                msg_send![class!(NSURL),fileURLWithPath:ns(&path.to_string_lossy())];
            let absolute: *mut Object = msg_send![url, absoluteString];
            let _: bool = msg_send![item,setString:absolute forType:ns("public.file-url")];
            let _: () = msg_send![filenames, addObject:ns(&path.to_string_lossy())];
            if paths.len() == 1 {
                let bytes = &bytes[0];
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
        let board: *mut Object = match name {
            Some(name) => msg_send![class!(NSPasteboard), pasteboardWithName:ns(name)],
            None => msg_send![class!(NSPasteboard), generalPasteboard],
        };
        let _: isize = msg_send![board, clearContents];
        let mut success: bool = msg_send![board,writeObjects:array];
        if paths.len() > 1 && success {
            // PowerPoint consumes the aggregate Finder-style file list. Individual
            // public.file-url items alone are ignored by its Paste command.
            success =
                msg_send![board, setPropertyList:filenames forType:ns("NSFilenamesPboardType")];
        }
        let _: () = msg_send![pool, drain];
        if success {
            Ok(())
        } else {
            Err("Unable to copy images.".into())
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use objc::{class, msg_send, runtime::Object, sel, sel_impl};

    #[test]
    fn clipboard_keeps_all_files_and_single_image_data() {
        let temp = tempfile::tempdir().unwrap();
        let paths: Vec<_> = ["triangle.svg", "equation.svg", "workflow.svg"]
            .iter()
            .map(|name| temp.path().join(name))
            .collect();
        for path in &paths {
            std::fs::write(path, b"<svg/>").unwrap();
        }
        let name = format!("Mathlidelux-test-{}", uuid::Uuid::new_v4());
        clipboard_to(&paths, Some(&name)).unwrap();
        unsafe {
            let pool: *mut Object = msg_send![class!(NSAutoreleasePool), new];
            let nsname: *mut Object = msg_send![class!(NSString), alloc];
            let nsname: *mut Object =
                msg_send![nsname, initWithBytes:name.as_ptr() length:name.len() encoding:4usize];
            let board: *mut Object = msg_send![class!(NSPasteboard), pasteboardWithName:nsname];
            let kind = b"NSFilenamesPboardType\0";
            let kind: *mut Object = msg_send![class!(NSString), stringWithUTF8String:kind.as_ptr()];
            let files: *mut Object = msg_send![board, propertyListForType:kind];
            assert!(!files.is_null());
            let count: usize = msg_send![files, count];
            assert_eq!(count, 3);
            for (i, path) in paths.iter().enumerate() {
                let file: *mut Object = msg_send![files, objectAtIndex:i];
                let value: *const std::ffi::c_char = msg_send![file, UTF8String];
                assert_eq!(
                    std::ffi::CStr::from_ptr(value).to_str().unwrap(),
                    path.to_str().unwrap()
                );
            }
            let items: *mut Object = msg_send![board, pasteboardItems];
            let count: usize = msg_send![items, count];
            assert_eq!(count, 3);
            let before: isize = msg_send![board, changeCount];
            assert!(clipboard_to(&[], Some(&name)).is_err());
            assert!(clipboard_to(
                &[paths[0].clone(), temp.path().join("missing.svg")],
                Some(&name)
            )
            .is_err());
            let after: isize = msg_send![board, changeCount];
            assert_eq!(
                before, after,
                "Invalid selections must preserve the clipboard"
            );
            clipboard_to(&paths[..1], Some(&name)).unwrap();
            let kind = b"public.svg-image\0";
            let kind: *mut Object = msg_send![class!(NSString), stringWithUTF8String:kind.as_ptr()];
            let data: *mut Object = msg_send![board, dataForType:kind];
            assert!(!data.is_null());
            let len: usize = msg_send![data, length];
            assert_eq!(len, 6);
            let _: () = msg_send![board, releaseGlobally];
            let _: () = msg_send![nsname, release];
            let _: () = msg_send![pool, drain];
        }
    }
}
#[cfg(not(target_os = "macos"))]
pub fn clipboard(_: &[PathBuf]) -> Result<()> {
    Err("Native image clipboard is currently supported on macOS.".into())
}
