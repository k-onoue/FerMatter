use crate::{compiler, files::*, scan, storage::Storage};
use serde_json::Value;
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::Mutex,
};
static EXPORT: Mutex<()> = Mutex::new(());

/// Resolve each export from persisted settings so every caller (including
/// batch copies and native drags) uses the same inheritance rules.
pub fn font_size(
    request: &Value,
    card: &Value,
    project: &Value,
    settings: &crate::settings::SettingsFiles,
) -> Result<f64> {
    if let Some(size) = request
        .as_f64()
        .or(card["exportFontSize"].as_f64())
        .or(project["exportFontSize"].as_f64())
    {
        return Ok(size);
    }
    Ok(settings.load("preferences.json")?["exportFontSize"]
        .as_f64()
        .unwrap_or(30.))
}

/// Card paths may come from the webview or stored metadata. Resolve them before
/// any lease, cache hit or read, and read only the resolved files.
pub fn image_file(
    store: &Storage,
    card: &Value,
    format: &str,
    point_size: f64,
    folder: &Path,
) -> Result<PathBuf> {
    let _permit = EXPORT.lock().unwrap();
    if !["svg", "png", "jpeg", "pdf"].contains(&format) {
        return Err("Unsupported image format.".into());
    }
    let source = store.artifact_file(Path::new(string(card, "svgPath")))?;
    let pdf = if format == "pdf" {
        Some(store.artifact_file(Path::new(string(card, "pdfPath")))?)
    } else {
        None
    };
    let _source_lease = crate::cache::Lease::new(folder, card);
    let svg = String::from_utf8(read_bounded(&source, ARTIFACT_LIMIT)?).map_err(err)?;
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
        id(&format!(
            "export-3|{svg}|{scale}|{format}|{}",
            card["restoreMetadata"]
        )),
        format
    ));
    fs::create_dir_all(folder).map_err(err)?;
    let dest = base.with_extension(if format == "jpeg" { "jpg" } else { format });
    let _lease = crate::cache::Lease::new(folder, &serde_json::json!(dest));
    if dest.is_file() {
        return Ok(dest);
    }
    let final_dest = dest;
    let staging = tempfile::tempdir().map_err(err)?;
    let base = staging.path().join(base.file_name().unwrap());
    let dest = base.with_extension(if format == "jpeg" { "jpg" } else { format });
    if let Some(source) = pdf {
        if (scale - 1.).abs() < 0.0001 {
            let mut input = regular_file(&source)?.take(ARTIFACT_LIMIT + 1);
            let mut output = fs::File::create(&dest).map_err(err)?;
            if std::io::copy(&mut input, &mut output).map_err(err)? > ARTIFACT_LIMIT {
                return Err("Export PDF exceeds the 100 MiB limit.".into());
            }
        } else {
            scale_pdf(&source, &dest, scale as f32)?;
        }
        crate::cache::publish(&[(dest, final_dest.clone())])?;
        return Ok(final_dest);
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
        let svg = if card["restoreMetadata"].is_null() {
            scaled
        } else {
            crate::roundtrip::embed(&scaled, &card["restoreMetadata"])?
        };
        atomic(&dest, svg.as_bytes())?;
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
    crate::cache::publish(&[(dest, final_dest.clone())])?;
    Ok(final_dest)
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
    const PROJECT: &str = "0123456789abcdef01234567";
    const SVG: &str = "<?xml version=\"1.0\"?><svg xmlns=\"http://www.w3.org/2000/svg\" width=\"20pt\" height=\"10pt\" viewBox=\"0 0 20 10\"><path d=\"M1 1h8v8H1z\"/></svg>";
    /// Storage with a rendered SVG in a project cache; returns it and the export folder.
    fn managed(svg: &str) -> (tempfile::TempDir, Storage, PathBuf, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let store = Storage::new(temp.path().join("data"), temp.path().join("legacy")).unwrap();
        let project = store.project_dir(PROJECT).unwrap();
        let source = project.join("Cache/source.svg");
        atomic(&source, svg.as_bytes()).unwrap();
        (temp, store, source, project.join("Exports"))
    }
    #[test]
    fn export_inherits_persisted_defaults_and_preserves_explicit_and_legacy_sizes() {
        let (temp, store, source, exports) = managed(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="20pt" height="10pt" viewBox="0 0 20 10"/>"#,
        );
        let settings = crate::settings::SettingsFiles::new(temp.path());
        let mut prefs = json!({"appearance":"light","autoSave":true});
        settings.initialize("preferences.json", &prefs).unwrap();
        let card = json!({"svgPath":source,"sourceFontSize":10});
        let project = json!({});
        let resolve = |request: &Value, card: &Value, project: &Value| {
            font_size(request, card, project, &settings).unwrap()
        };
        let size = resolve(&Value::Null, &card, &project);
        assert_eq!(size, 30.);
        let svg = image_file(&store, &card, "svg", size, &exports).unwrap();
        assert!(read(&svg).unwrap().contains("width=\"60pt\""));
        let snapshot = settings.snapshot("preferences.json", false).unwrap();
        prefs["exportFontSize"] = json!(31.25);
        settings
            .save("preferences.json", &prefs, string(&snapshot, "revision"))
            .unwrap();
        assert_eq!(resolve(&Value::Null, &card, &project), 31.25);
        assert_eq!(
            resolve(&Value::Null, &card, &json!({"exportFontSize":24})),
            24.
        );
        assert_eq!(
            resolve(
                &Value::Null,
                &json!({"exportFontSize":18.5}),
                &json!({"exportFontSize":24})
            ),
            18.5
        );
        assert_eq!(
            resolve(&json!(12.5), &json!({"exportFontSize":18}), &project),
            12.5
        );
        assert_eq!(
            resolve(&Value::Null, &json!({"exportFontSize":0}), &project),
            0.
        );
        assert_eq!(
            resolve(&Value::Null, &card, &json!({"exportFontSize":0})),
            0.
        );
        assert_eq!(resolve(&json!(0), &card, &project), 0.);
    }
    #[test]
    fn scales_svg_with_xml_declaration_without_changing_viewbox() {
        let (_temp, store, source, folder) = managed(SVG);
        let card = json!({"svgPath":source,"sourceFontSize":10});
        let svg = image_file(&store, &card, "svg", 20., &folder).unwrap();
        let text = fs::read_to_string(svg).unwrap();
        assert!(text.contains("width=\"40pt\""));
        assert!(text.contains("height=\"20pt\""));
        assert!(text.contains("viewBox=\"0 0 20 10\""));
        let png = image_file(&store, &card, "png", 20., &folder).unwrap();
        let jpeg = image_file(&store, &card, "jpeg", 20., &folder).unwrap();
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
    #[test]
    fn image_file_exports_managed_artifacts() {
        let (_temp, store, source, exports) = managed(SVG);
        let pdf = source.with_extension("pdf");
        atomic(&pdf, b"%PDF-1.4 rendered").unwrap();
        let card = json!({"svgPath":source,"pdfPath":pdf,"sourceFontSize":10});
        for format in ["svg", "png", "jpeg", "pdf"] {
            let path = image_file(&store, &card, format, 10., &exports).unwrap();
            assert!(path.starts_with(&exports), "{format}");
            assert!(path.metadata().unwrap().len() > 0, "{format}");
        }
        let copied = image_file(&store, &card, "pdf", 10., &exports).unwrap();
        assert_eq!(fs::read(copied).unwrap(), b"%PDF-1.4 rendered");
        let instant = store.home.join("Instant/Cache/a.svg");
        atomic(&instant, SVG.as_bytes()).unwrap();
        let card = json!({"svgPath":instant,"sourceFontSize":10});
        let folder = store.home.join("Instant/Exports");
        assert!(image_file(&store, &card, "svg", 10., &folder).is_ok());
    }
    #[test]
    fn image_file_rejects_svg_path_outside_storage() {
        let (temp, store, _, exports) = managed(SVG);
        let outside = temp.path().join("outside.svg");
        fs::write(&outside, SVG).unwrap();
        let settings = store.home.join("Settings/preferences.svg");
        fs::write(&settings, SVG).unwrap();
        for path in [outside, settings] {
            let card = json!({"svgPath":path,"pdfPath":path,"sourceFontSize":10});
            for format in ["svg", "png", "jpeg", "pdf"] {
                assert!(image_file(&store, &card, format, 10., &exports).is_err());
            }
        }
        assert!(!exports.exists());
    }
    #[test]
    fn image_file_rejects_pdf_path_outside_storage_for_pdf_export() {
        let (temp, store, source, exports) = managed(SVG);
        let secret = temp.path().join("secret.pdf");
        fs::write(&secret, b"%PDF-1.4 secret").unwrap();
        let metadata = store.project_dir(PROJECT).unwrap().join("project.json");
        fs::write(&metadata, b"{}").unwrap();
        for path in [&secret, &metadata] {
            let card = json!({"svgPath":source,"pdfPath":path,"sourceFontSize":10});
            // Both the plain copy (scale 1) and the scaled path are refused.
            for size in [10., 20.] {
                assert!(image_file(&store, &card, "pdf", size, &exports).is_err());
            }
        }
        assert!(!exports.exists());
    }
    #[test]
    fn image_file_rejects_forged_pdf_path_even_when_the_export_is_cached() {
        let (temp, store, source, exports) = managed(SVG);
        let pdf = source.with_extension("pdf");
        atomic(&pdf, b"%PDF-1.4 rendered").unwrap();
        let card = json!({"svgPath":source,"pdfPath":pdf,"sourceFontSize":10});
        let cached = image_file(&store, &card, "pdf", 10., &exports).unwrap();
        let secret = temp.path().join("secret.pdf");
        fs::write(&secret, b"%PDF-1.4 secret").unwrap();
        let forged = json!({"svgPath":source,"pdfPath":secret,"sourceFontSize":10});
        assert!(image_file(&store, &forged, "pdf", 10., &exports).is_err());
        assert_eq!(fs::read(cached).unwrap(), b"%PDF-1.4 rendered");
    }
    #[test]
    fn image_file_ignores_pdf_path_for_svg_png_and_jpeg_exports() {
        let (temp, store, source, exports) = managed(SVG);
        let card =
            json!({"svgPath":source,"pdfPath":temp.path().join("missing.pdf"),"sourceFontSize":10});
        for format in ["svg", "png", "jpeg"] {
            assert!(image_file(&store, &card, format, 10., &exports).is_ok());
        }
    }
}
