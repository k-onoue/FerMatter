//! Windows clipboard representations: Explorer file drops, SVG and bitmap data.
use crate::files::*;
use std::{ffi::c_void, os::windows::ffi::OsStrExt, path::PathBuf};

type Handle = *mut c_void;
#[link(name = "user32")]
unsafe extern "system" {
    fn OpenClipboard(owner: Handle) -> i32;
    fn CloseClipboard() -> i32;
    fn EmptyClipboard() -> i32;
    fn SetClipboardData(format: u32, memory: Handle) -> Handle;
    fn GetClipboardData(format: u32) -> Handle;
    fn RegisterClipboardFormatW(name: *const u16) -> u32;
    fn CreateWindowExW(
        ex_style: u32,
        class: *const u16,
        title: *const u16,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: Handle,
        menu: Handle,
        instance: Handle,
        parameter: Handle,
    ) -> Handle;
    fn DestroyWindow(window: Handle) -> i32;
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GlobalAlloc(flags: u32, bytes: usize) -> Handle;
    fn GlobalLock(memory: Handle) -> Handle;
    fn GlobalUnlock(memory: Handle) -> i32;
    fn GlobalFree(memory: Handle) -> Handle;
    fn GlobalSize(memory: Handle) -> usize;
}
#[link(name = "shell32")]
unsafe extern "system" {
    fn DragQueryFileW(drop: Handle, index: u32, buffer: *mut u16, size: u32) -> u32;
}
struct Clipboard(Handle);
impl Clipboard {
    fn open() -> Result<Self> {
        let class: Vec<_> = "STATIC".encode_utf16().chain(Some(0)).collect();
        // A message-only owner also works when a global shortcut copies from a
        // hidden app. OpenClipboard(NULL) cannot publish after EmptyClipboard.
        let owner = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                std::ptr::null(),
                0,
                0,
                0,
                0,
                0,
                -3isize as Handle,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if owner.is_null() {
            return Err("Unable to create a clipboard owner.".into());
        }
        for _ in 0..10 {
            if unsafe { OpenClipboard(owner) } != 0 {
                return Ok(Self(owner));
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        unsafe {
            DestroyWindow(owner);
        }
        Err("The clipboard is busy. Try copying again.".into())
    }
}
impl Drop for Clipboard {
    fn drop(&mut self) {
        unsafe {
            CloseClipboard();
            DestroyWindow(self.0);
        }
    }
}
struct Memory(Handle);
impl Memory {
    fn new(bytes: &[u8]) -> Result<Self> {
        let memory = Self(unsafe { GlobalAlloc(0x0002, bytes.len()) }); // GMEM_MOVEABLE
        if memory.0.is_null() {
            return Err("Unable to allocate clipboard data.".into());
        }
        let target = unsafe { GlobalLock(memory.0) };
        if target.is_null() {
            return Err("Unable to lock clipboard data.".into());
        }
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), target.cast::<u8>(), bytes.len());
            GlobalUnlock(memory.0);
        }
        Ok(memory)
    }
    fn publish(mut self, format: u32) -> Result<()> {
        if unsafe { SetClipboardData(format, self.0) }.is_null() {
            return Err("Unable to copy images to the Windows clipboard.".into());
        }
        self.0 = std::ptr::null_mut(); // The system now owns this allocation.
        Ok(())
    }
}
impl Drop for Memory {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                GlobalFree(self.0);
            }
        }
    }
}
fn format(name: &str) -> Result<u32> {
    let name: Vec<_> = name.encode_utf16().chain(Some(0)).collect();
    let format = unsafe { RegisterClipboardFormatW(name.as_ptr()) };
    if format == 0 {
        Err("Unable to register a clipboard image format.".into())
    } else {
        Ok(format)
    }
}
fn file_drop(paths: &[PathBuf]) -> Vec<u8> {
    // DROPFILES: offset, POINT, non-client flag, UTF-16 flag.
    let mut bytes = Vec::from(20u32.to_le_bytes());
    bytes.extend_from_slice(&[0; 12]);
    bytes.extend_from_slice(&1u32.to_le_bytes());
    for path in paths {
        for unit in std::ffi::OsStr::new(&windows_path(path))
            .encode_wide()
            .chain(Some(0))
        {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
    }
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes
}
fn bitmap(bytes: &[u8]) -> Result<Vec<u8>> {
    let image = image::load_from_memory(bytes).map_err(err)?.into_rgba8();
    let (width, height) = image.dimensions();
    if width as u64 * height as u64 > 16_000_000 {
        return Err("Clipboard bitmap exceeds 16 megapixels.".into());
    }
    let pixels = image.into_raw();
    let mut dib = vec![0u8; 124]; // BITMAPV5HEADER, top-down BGRA.
    dib[0..4].copy_from_slice(&124u32.to_le_bytes());
    dib[4..8].copy_from_slice(&(width as i32).to_le_bytes());
    dib[8..12].copy_from_slice(&(-(height as i32)).to_le_bytes());
    dib[12..14].copy_from_slice(&1u16.to_le_bytes());
    dib[14..16].copy_from_slice(&32u16.to_le_bytes());
    dib[16..20].copy_from_slice(&3u32.to_le_bytes()); // BI_BITFIELDS
    dib[20..24].copy_from_slice(&(pixels.len() as u32).to_le_bytes());
    for (offset, mask) in [
        (40, 0x00ff0000u32),
        (44, 0x0000ff00),
        (48, 0x000000ff),
        (52, 0xff000000),
        (56, 0x73524742),
    ] {
        dib[offset..offset + 4].copy_from_slice(&mask.to_le_bytes());
    }
    for pixel in pixels.as_chunks::<4>().0 {
        dib.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
    }
    Ok(dib)
}
pub fn clipboard(paths: &[PathBuf]) -> Result<()> {
    if paths.is_empty() {
        return Err("No images selected.".into());
    }
    for path in paths {
        if regular_file(path)?.metadata().map_err(err)?.len() > ARTIFACT_LIMIT {
            return Err("Image exceeds the 100 MB clipboard limit.".into());
        }
    }
    // Prepare all allocations before replacing the user's current clipboard.
    let mut data = vec![(15, Memory::new(&file_drop(paths))?)]; // CF_HDROP
    if paths.len() == 1 {
        let bytes = read_bounded(&paths[0], ARTIFACT_LIMIT)?;
        let extension = paths[0]
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let png = match extension.as_str() {
            "svg" => {
                data.push((format("image/svg+xml")?, Memory::new(&bytes)?));
                let output = tempfile::Builder::new()
                    .suffix(".png")
                    .tempfile()
                    .map_err(err)?;
                crate::compiler::raster(&bytes, output.path(), 900)?;
                Some(read_bounded(output.path(), ARTIFACT_LIMIT)?)
            }
            "png" => Some(bytes),
            "jpg" | "jpeg" => {
                data.push((17, Memory::new(&bitmap(&bytes)?)?));
                None
            }
            "pdf" => {
                data.push((format("application/pdf")?, Memory::new(&bytes)?));
                None
            }
            _ => None,
        };
        if let Some(png) = png {
            data.push((format("PNG")?, Memory::new(&png)?));
            data.push((17, Memory::new(&bitmap(&png)?)?)); // CF_DIBV5
        }
    }
    let _clipboard = Clipboard::open()?;
    if unsafe { EmptyClipboard() } == 0 {
        return Err("Unable to clear the clipboard.".into());
    }
    for (format, memory) in data {
        memory.publish(format)?;
    }
    Ok(())
}
pub fn import_svg() -> Result<Vec<String>> {
    let _clipboard = Clipboard::open()?;
    for name in ["image/svg+xml", "image/svg+xml;charset=utf-8"] {
        let memory = unsafe { GetClipboardData(format(name)?) };
        if !memory.is_null() {
            let size = unsafe { GlobalSize(memory) };
            if size > 16 * 1024 * 1024 {
                return Err("SVG clipboard exceeds 16 MB.".into());
            }
            let pointer = unsafe { GlobalLock(memory) };
            if pointer.is_null() {
                return Err("Unable to read clipboard SVG.".into());
            }
            let bytes = unsafe { std::slice::from_raw_parts(pointer.cast::<u8>(), size) };
            let logical = bytes
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(bytes.len());
            let result = crate::svg_payload::decode("image/svg+xml", &bytes[..logical]);
            unsafe {
                GlobalUnlock(memory);
            }
            return result;
        }
    }
    let drop = unsafe { GetClipboardData(15) };
    if !drop.is_null() {
        let count = unsafe { DragQueryFileW(drop, u32::MAX, std::ptr::null_mut(), 0) };
        if count > 16 {
            return Err("Open between 1 and 16 SVG files.".into());
        }
        let mut sources = Vec::new();
        for index in 0..count {
            let size = unsafe { DragQueryFileW(drop, index, std::ptr::null_mut(), 0) };
            let mut path = vec![0u16; size as usize + 1];
            unsafe {
                DragQueryFileW(drop, index, path.as_mut_ptr(), size + 1);
            }
            let path = PathBuf::from(String::from_utf16(&path[..size as usize]).map_err(err)?);
            sources.push(crate::roundtrip::read_svg(&path)?);
        }
        if !sources.is_empty() {
            return Ok(sources);
        }
    }
    Err("The clipboard does not contain SVG artwork. Copy SVG or SVG files first.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explorer_file_drop_uses_utf16_and_double_null_termination() {
        let bytes = file_drop(&[
            PathBuf::from(r"C:\図\one.svg"),
            PathBuf::from(r"C:\two.svg"),
        ]);
        assert_eq!(&bytes[..4], &20u32.to_le_bytes());
        assert_eq!(&bytes[16..20], &1u32.to_le_bytes());
        let names: Vec<_> = bytes[20..]
            .chunks_exact(2)
            .map(|v| u16::from_le_bytes([v[0], v[1]]))
            .collect();
        assert_eq!(
            String::from_utf16(&names).unwrap(),
            "C:\\図\\one.svg\0C:\\two.svg\0\0"
        );
    }
    #[test]
    #[ignore = "uses the current Windows desktop clipboard"]
    fn windows_desktop_clipboard_round_trip_keeps_svg_and_image_formats() {
        let directory = tempfile::tempdir().unwrap();
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="16" height="16" fill="blue"/></svg>"#;
        let path = directory.path().join("synthetic.svg");
        std::fs::write(&path, svg).unwrap();
        clipboard(std::slice::from_ref(&path)).unwrap();
        assert_eq!(import_svg().unwrap(), vec![svg]);
        {
            let _clipboard = Clipboard::open().unwrap();
            for representation in [
                15,
                17,
                format("PNG").unwrap(),
                format("image/svg+xml").unwrap(),
            ] {
                assert!(!unsafe { GetClipboardData(representation) }.is_null());
            }
        }
        assert!(clipboard(&[path, directory.path().join("missing.svg")]).is_err());
        assert_eq!(import_svg().unwrap(), vec![svg]);
    }
}
