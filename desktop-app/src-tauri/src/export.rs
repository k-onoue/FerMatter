use crate::{compiler, files::*, scan};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};
static EXPORT: Mutex<()> = Mutex::new(());
pub fn image_file(card: &Value, format: &str, point_size: f64, folder: &Path) -> Result<PathBuf> {
    let _permit = EXPORT.lock().unwrap();
    if !["svg", "png", "jpeg", "pdf"].contains(&format) {
        return Err("Unsupported image format.".into());
    }
    let source = Path::new(string(card, "svgPath"));
    let svg = String::from_utf8(read_bounded(source, ARTIFACT_LIMIT)?).map_err(err)?;
    let original = card["sourceFontSize"].as_f64().unwrap_or(11.);
    let scale = if point_size > 0. {
        point_size / original
    } else {
        1.
    };
    if !scale.is_finite() || scale <= 0. || scale > 100. {
        return Err("Invalid export size.".into());
    }
    let base = folder.join(format!(
        "{}-{}",
        id(&format!("export-2|{svg}|{scale}|{format}")),
        format
    ));
    fs::create_dir_all(folder).map_err(err)?;
    let dest = base.with_extension(if format == "jpeg" { "jpg" } else { format });
    if dest.is_file() {
        return Ok(dest);
    }
    if format == "pdf" {
        let source = Path::new(string(card, "pdfPath"));
        if (scale - 1.).abs() < 0.0001 {
            fs::copy(source, &dest).map_err(err)?;
        } else {
            scale_pdf(source, &dest, scale as f32)?;
        }
        return Ok(dest);
    }
    let root_start = svg.find("<svg").ok_or("Invalid SVG")?;
    let root_end = root_start + svg[root_start..].find('>').ok_or("Invalid SVG")?;
    let root = &svg[..root_end];
    let dim = scan::re(r#"\b(width|height)="([0-9.]+)(pt|px)?""#);
    let header = dim.replace_all(root, |c: &regex::Captures| {
        format!(
            "{}=\"{}{}\"",
            &c[1],
            c[2].parse::<f64>().unwrap_or(1.) * scale,
            c.get(3).map(|m| m.as_str()).unwrap_or("")
        )
    });
    let scaled = format!("{}{}", header, &svg[root_end..]);
    if format == "svg" {
        atomic(&dest, scaled.as_bytes())?;
    } else {
        let options = resvg::usvg::Options::default();
        let tree = resvg::usvg::Tree::from_data(scaled.as_bytes(), &options).map_err(err)?;
        let longest = tree.size().width().max(tree.size().height());
        let width = (longest * 3.).round().clamp(1., 16000.) as u32;
        let png = base.with_extension("png");
        compiler::raster(scaled.as_bytes(), &png, width)?;
        if format == "jpeg" {
            let mut reader = image::ImageReader::open(&png).map_err(err)?;
            let mut limits = image::Limits::default();
            limits.max_image_width = Some(16000);
            limits.max_image_height = Some(16000);
            limits.max_alloc = Some(64 * 1024 * 1024);
            reader.limits(limits);
            let rgba = reader.decode().map_err(err)?.to_rgba8();
            let mut rgb = image::RgbImage::new(rgba.width(), rgba.height());
            for (x, y, p) in rgba.enumerate_pixels() {
                let a = p.0[3] as f32 / 255.;
                rgb.put_pixel(
                    x,
                    y,
                    image::Rgb(
                        [0, 1, 2].map(|i| (p.0[i] as f32 * a + 255. * (1. - a)).round() as u8),
                    ),
                );
            }
            rgb.save(&dest).map_err(err)?;
        }
    }
    Ok(dest)
}

fn scale_pdf(source: &Path, dest: &Path, scale: f32) -> Result<()> {
    use lopdf::{Dictionary, Document, Object, Stream};
    let mut document = Document::load_mem(&read_bounded(source, ARTIFACT_LIMIT)?).map_err(err)?;
    for (_, page_id) in document.get_pages() {
        let mut ancestor = page_id;
        let bounds = loop {
            let page = document.get_dictionary(ancestor).map_err(err)?;
            if let Ok(bounds) = page.get(b"MediaBox") {
                let bounds = document
                    .dereference(bounds)
                    .map_err(err)?
                    .1
                    .as_array()
                    .map_err(err)?;
                break bounds
                    .iter()
                    .map(|n| n.as_float().map_err(err))
                    .collect::<Result<Vec<_>>>()?;
            }
            ancestor = page
                .get(b"Parent")
                .and_then(Object::as_reference)
                .map_err(err)?;
        };
        if bounds.len() != 4 {
            return Err("Invalid PDF page bounds".into());
        }
        let data = document.get_page_content(page_id).map_err(err)?;
        let mut content = format!(
            "q\n{scale} 0 0 {scale} {} {} cm\n",
            -bounds[0] * scale,
            -bounds[1] * scale
        )
        .into_bytes();
        content.extend(data);
        content.extend_from_slice(b"\nQ\n");
        let stream = document.add_object(Stream::new(Dictionary::new(), content));
        let page = document
            .get_object_mut(page_id)
            .and_then(Object::as_dict_mut)
            .map_err(err)?;
        let rect = vec![
            0.into(),
            0.into(),
            Object::Real((bounds[2] - bounds[0]) * scale),
            Object::Real((bounds[3] - bounds[1]) * scale),
        ];
        page.set("MediaBox", rect.clone());
        page.set("CropBox", rect);
        page.set("Contents", Object::Reference(stream));
    }
    document.compress();
    document.save(dest).map_err(err)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn scales_svg_with_xml_declaration_without_changing_viewbox() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.svg");
        fs::write(&source, "<?xml version=\"1.0\"?><svg xmlns=\"http://www.w3.org/2000/svg\" width=\"20pt\" height=\"10pt\" viewBox=\"0 0 20 10\"><path d=\"M1 1h8v8H1z\"/></svg>").unwrap();
        let card = json!({"svgPath":source,"sourceFontSize":10});
        let folder = temp.path().join("exports");
        let svg = image_file(&card, "svg", 20., &folder).unwrap();
        let text = fs::read_to_string(svg).unwrap();
        assert!(text.contains("width=\"40pt\""));
        assert!(text.contains("height=\"20pt\""));
        assert!(text.contains("viewBox=\"0 0 20 10\""));
        let png = image_file(&card, "png", 20., &folder).unwrap();
        let jpeg = image_file(&card, "jpeg", 20., &folder).unwrap();
        let png = image::open(png).unwrap().to_rgba8();
        let jpeg = image::open(jpeg).unwrap().to_rgb8();
        assert_eq!(png.dimensions(), jpeg.dimensions());
        assert_eq!(png.get_pixel(png.width() - 1, png.height() - 1).0[3], 0);
        assert!(jpeg
            .get_pixel(jpeg.width() - 1, jpeg.height() - 1)
            .0
            .iter()
            .all(|c| *c > 245));
    }
}
