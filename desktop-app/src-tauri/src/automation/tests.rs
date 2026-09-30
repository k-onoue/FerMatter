use super::*;
fn fixture() -> (tempfile::TempDir, Storage, SettingsFiles, Value) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("source");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("main.typ"), "= Model\n$ x + 1 $\n").unwrap();
    let store = Storage::new(tmp.path().join("data"), tmp.path().join("legacy")).unwrap();
    let settings = SettingsFiles::new(&store.home);
    let mut p = store.open(&root, Some("main.typ")).unwrap();
    p["cards"] = json!([{"id":"one","name":"Equation","tex":"x + 1","kind":"block","file":"main.typ","origin":"source","outlinePath":["heading"]}]);
    p["outline"] = json!([{"id":"heading","title":"Model","level":1,"file":"main.typ","line":1,"path":["heading"]}]);
    store.save(string(&p, "id"), &p).unwrap();
    (tmp, store, settings, p)
}
#[test]
fn searches_saved_source_and_heading_with_validated_pagination() {
    let (_tmp, store, settings, p) = fixture();
    let service = Service {
        store: &store,
        settings: &settings,
    };
    let c = compiler::Cancellation::default();
    let result = service
        .call(
            "search_equations",
            json!({"projectId":p["id"],"query":"model"}),
            &c,
            &mut |_| {},
        )
        .unwrap();
    assert_eq!(result["total"], 1);
    assert_eq!(result["equations"][0]["id"], "one");
    assert_eq!(
        result["revision"],
        revision(&store.load(string(&p, "id")).unwrap())
    );
    assert_eq!(
        service
            .call("list_projects", json!({"limit":0}), &c, &mut |_| {})
            .unwrap_err()
            .code,
        "invalid_arguments"
    );
    assert_eq!(
        service
            .call(
                "get_equation",
                json!({"projectId":p["id"],"equationId":"missing"}),
                &c,
                &mut |_| {}
            )
            .unwrap_err()
            .code,
        "equation_not_found"
    );
}
#[test]
fn rejects_conflicts_and_cancelled_calls_before_rendering() {
    let (_tmp, store, settings, p) = fixture();
    let service = Service {
        store: &store,
        settings: &settings,
    };
    let c = compiler::Cancellation::default();
    let args = json!({"projectId":p["id"],"equationId":"one","expectedRevision":"stale","requestId":"request","name":"Variant","source":"x + 2"});
    assert_eq!(
        service
            .call("create_variant", args, &c, &mut |_| {})
            .unwrap_err()
            .code,
        "project_conflict"
    );
    assert_eq!(
        store.load(string(&p, "id")).unwrap()["cards"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    c.cancel();
    assert_eq!(
        service
            .call("list_projects", json!({}), &c, &mut |_| {})
            .unwrap_err()
            .code,
        "cancelled"
    );
}
#[test]
fn stale_gui_save_preserves_unseen_variants_but_allows_seen_deletion() {
    let original = json!({"cards":[{"id":"source","tex":"saved"}],"automationGeneration":0});
    let stored = json!({"cards":[{"id":"source","tex":"saved"},{"id":"api","automationGeneration":1}],"automationGeneration":1});
    let mut incoming = original.clone();
    incoming["cards"][0]["tex"] = json!("draft");
    let merged = merge_unseen_variants(&incoming, &stored);
    assert_eq!(merged["cards"][0]["tex"], "draft");
    assert_eq!(merged["cards"].as_array().unwrap().len(), 2);
    incoming["automationGeneration"] = json!(1);
    assert_eq!(
        merge_unseen_variants(&incoming, &stored)["cards"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn outputs_cannot_write_inside_sources_or_symlinked_sources() {
    let (tmp, store, settings, p) = fixture();
    let service = Service {
        store: &store,
        settings: &settings,
    };
    assert!(service
        .output_directory(Path::new(string(&p, "root")))
        .is_err());
    assert!(service
        .output_directory(&tmp.path().join("missing"))
        .is_err());
    assert!(!tmp.path().join("missing").exists());
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(string(&p, "root"), tmp.path().join("alias")).unwrap();
        assert!(service.output_directory(&tmp.path().join("alias")).is_err());
    }
}
#[test]
fn schemas_and_typed_arguments_reject_unknown_fields() {
    assert_eq!(tools().len(), 8);
    for tool in tools() {
        assert_eq!(tool.input_schema["additionalProperties"], false);
    }
    assert!(
        parse::<schema::Render>(json!({"source":"x","outputDirectory":"/tmp","format":"gif"}))
            .is_err()
    );
    assert!(
        parse::<schema::Render>(json!({"source":"x","outputDirectory":"/tmp","delete":true}))
            .is_err()
    );
    assert!(schema::validate_color("#ff0000").is_ok());
    assert!(schema::validate_color("#ff0000;rm").is_err());
}

#[test]
fn inherits_numeric_or_original_size_without_accepting_explicit_zero() {
    let (_tmp, store, settings, _p) = fixture();
    let service = Service {
        store: &store,
        settings: &settings,
    };
    assert_eq!(service.size(None, None, None).unwrap(), 30.);
    assert_eq!(
        service
            .size(None, Some(&json!({"exportFontSize":0})), None)
            .unwrap(),
        0.
    );
    assert_eq!(
        service
            .size(
                None,
                Some(&json!({"exportFontSize":20})),
                Some(&json!({"exportFontSize":40}))
            )
            .unwrap(),
        40.
    );
    assert_eq!(
        service
            .size(Some(32.), Some(&json!({"exportFontSize":0})), None)
            .unwrap(),
        32.
    );
    assert!(service.size(Some(0.), None, None).is_err());
}
