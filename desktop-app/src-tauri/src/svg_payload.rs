//! Bounded, inert decoding of clipboard/drag representations; never fetch URLs.
use crate::{files::*, roundtrip::SVG_LIMIT};
use base64::Engine;
use std::io::{Cursor, Read};

pub const BATCH_LIMIT: usize = 64 * 1024 * 1024;
pub fn decode(kind: &str, bytes: &[u8]) -> Result<Vec<String>> {
    if bytes.len() > BATCH_LIMIT {
        return Err("Dropped data exceeds 64 MB.".into());
    }
    let kind = kind.to_ascii_lowercase();
    if kind.contains("svg") {
        if bytes.len() as u64 > SVG_LIMIT {
            return Err("SVG exceeds 16 MB.".into());
        }
        return Ok(vec![String::from_utf8(bytes.to_vec()).map_err(err)?]);
    }
    if (kind.contains("microsoft") || kind.contains("office") || kind.contains("powerpoint"))
        && bytes.starts_with(b"PK\x03\x04")
    {
        return office_zip(bytes);
    }
    if kind.contains("html") || kind.contains("text") || kind.contains("string") {
        let Ok(text) = std::str::from_utf8(bytes) else {
            return Ok(vec![]);
        };
        return html(text);
    }
    Ok(vec![])
}
fn html(text: &str) -> Result<Vec<String>> {
    let lower = text.to_ascii_lowercase();
    let mut output = Vec::new();
    let mut cursor = 0;
    // Only local inline SVG and data: URLs. Never follow img/src file or network URLs.
    while let Some(start) = lower[cursor..].find("<svg") {
        let start = cursor + start;
        let Some(end) = lower[start..].find("</svg>") else {
            break;
        };
        let end = start + end + 6;
        push(&mut output, text[start..end].to_owned())?;
        cursor = end;
    }
    if !output.is_empty() {
        return Ok(output);
    }
    let re = crate::scan::re(
        r#"(?i)data:image/svg\+xml(?:;charset=[^;,\s\"']+)?(;base64)?,([^\s\"'<>]+)"#,
    );
    for captures in re.captures_iter(text) {
        let data = if captures.get(1).is_some() {
            base64::engine::general_purpose::STANDARD
                .decode(&captures[2])
                .map_err(err)?
        } else {
            percent_decode(&captures[2])?
        };
        push(&mut output, String::from_utf8(data).map_err(err)?)?;
    }
    Ok(output)
}
fn percent_decode(text: &str) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let input = text.as_bytes();
    let mut index = 0;
    while index < input.len() {
        if input[index] == b'%' {
            let encoded = input
                .get(index + 1..index + 3)
                .ok_or("Invalid SVG data URL.")?;
            let hex = std::str::from_utf8(encoded).map_err(err)?;
            bytes.push(u8::from_str_radix(hex, 16).map_err(err)?);
            index += 3;
        } else {
            bytes.push(input[index]);
            index += 1;
        }
    }
    Ok(bytes)
}
fn push(output: &mut Vec<String>, value: String) -> Result<()> {
    if value.len() as u64 > SVG_LIMIT {
        return Err("SVG exceeds 16 MB.".into());
    }
    if output.len() >= 16
        || output.iter().map(String::len).sum::<usize>() + value.len() > BATCH_LIMIT
    {
        return Err("Open up to 16 SVGs totalling at most 64 MB.".into());
    }
    output.push(value);
    Ok(())
}
fn office_zip(bytes: &[u8]) -> Result<Vec<String>> {
    // ZipArchive allocates directory entries eagerly. Check the small ordinary
    // Office ZIP directory before constructing it; ZIP64 is unnecessary here.
    let start = bytes.len().saturating_sub(65_557);
    let end = bytes[start..]
        .windows(22)
        .enumerate()
        .rev()
        .find_map(|(offset, record)| {
            if !record.starts_with(b"PK\x05\x06") {
                return None;
            }
            let position = start + offset;
            let comment = u16::from_le_bytes([record[20], record[21]]) as usize;
            (position + 22 + comment == bytes.len()).then_some(position)
        })
        .ok_or("Office payload has no valid ZIP directory.")?;
    let directory = &bytes[end..end + 22];
    let entries = u16::from_le_bytes([directory[10], directory[11]]) as usize;
    if directory[4..8] != [0, 0, 0, 0]
        || directory[8..10] != directory[10..12]
        || entries > 4096
        || (end >= 20 && &bytes[end - 20..end - 16] == b"PK\x06\x07")
    {
        return Err("Office payload uses an unsupported or oversized ZIP directory.".into());
    }
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(err)?;
    if archive.len() > 4096 {
        return Err("Office payload contains too many entries.".into());
    }
    let mut output = Vec::new();
    for index in 0..archive.len() {
        let file = archive.by_index(index).map_err(err)?;
        if !file.name().map_err(err)?.to_ascii_lowercase().ends_with(".svg") || file.is_dir() {
            continue;
        }
        if file.size() > SVG_LIMIT {
            return Err("SVG exceeds 16 MB.".into());
        }
        let mut text = String::new();
        file.take(SVG_LIMIT + 1)
            .read_to_string(&mut text)
            .map_err(err)?;
        push(&mut output, text)?;
    }
    Ok(output)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inline_and_data_uri_are_decoded_without_network_access() {
        assert_eq!(
            decode("public.html", b"<img src='https://private.test/x.svg'>").unwrap(),
            Vec::<String>::new()
        );
        let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\"><metadata>test</metadata></svg>";
        assert_eq!(
            decode("public.html", format!("<div>{svg}</div>").as_bytes()).unwrap(),
            vec![svg]
        );
        let encoded = base64::engine::general_purpose::STANDARD.encode(svg);
        assert_eq!(
            decode(
                "public.html",
                format!("<img src='data:image/svg+xml;base64,{encoded}'>").as_bytes()
            )
            .unwrap(),
            vec![svg]
        );
        assert_eq!(
            decode(
                "public.utf8-plain-text",
                b"data:image/svg+xml,%3Csvg%3E%3C/svg%3E"
            )
            .unwrap(),
            vec!["<svg></svg>"]
        );
    }
    #[test]
    fn office_zip_preserves_svg_bytes_and_ignores_other_entries() {
        use std::io::Write;
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file(
                "ppt/media/image1.svg",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        writer.write_all(b"<svg>recipe</svg>").unwrap();
        writer
            .start_file(
                "ppt/slides/slide1.xml",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        writer.write_all(b"PRIVATE BODY").unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        assert_eq!(
            decode("com.microsoft.PowerPoint", &bytes).unwrap(),
            vec!["<svg>recipe</svg>"]
        );
    }
    #[test]
    fn oversized_zip_directory_is_rejected_before_archive_allocation() {
        let mut zip = b"PK\x03\x04PK\x05\x06".to_vec();
        zip.extend_from_slice(&[0; 18]);
        zip[12..16].copy_from_slice(&[255, 255, 255, 255]);
        assert!(decode("com.microsoft.Art--GVML-ClipFormat", &zip)
            .unwrap_err()
            .contains("oversized ZIP directory"));
    }
    #[test]
    fn oversized_and_excessive_inputs_are_rejected() {
        assert!(decode("public.svg-image", &vec![b'x'; SVG_LIMIT as usize + 1]).is_err());
        assert!(decode("public.html", "<svg></svg>".repeat(17).as_bytes()).is_err());
    }
}
