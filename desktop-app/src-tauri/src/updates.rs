//! Only reviewed stable artifacts from the public distribution repository are eligible.
//! The WebView supplies actions, never URLs, signatures, paths or installer arguments.
use crate::{
    files::{err, json_write, Result},
    lifecycle,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};
use tokio::sync::oneshot;
use url::Url;

const FEED: &str = "https://fermatter.github.io/updates/stable.json";
const MAX_PACKAGE: u64 = 1_000_000_000;
const DAY: u64 = 24 * 60 * 60;

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Preferences {
    automatic: bool,
    last_check: u64,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    revision: u64,
    current_version: String,
    phase: &'static str,
    automatic: bool,
    last_check: u64,
    version: Option<String>,
    notes: Option<String>,
    downloaded: u64,
    total: u64,
    error: Option<String>,
    install_available: bool,
}
struct Candidate {
    update: Update,
    size: u64,
    sha256: String,
    bytes: Option<Vec<u8>>,
}
struct Inner {
    snapshot: Snapshot,
    preferences: Preferences,
    preferences_error: Option<String>,
    candidate: Option<Candidate>,
    cancel: Option<oneshot::Sender<()>>,
    operation: u64,
}
pub struct Updates {
    path: PathBuf,
    inner: Mutex<Inner>,
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
impl Updates {
    pub fn new(home: &Path, version: String) -> Self {
        let path = home.join("UpdatePreferences.json");
        let loaded = match std::fs::File::open(&path).and_then(|file| {
            let mut bytes = Vec::new();
            file.take(4097).read_to_end(&mut bytes)?;
            Ok(bytes)
        }) {
            Ok(bytes) if bytes.len() <= 4096 => serde_json::from_slice(&bytes).map_err(err),
            Ok(_) => {
                Err("Update preferences are too large; the original file has been kept.".into())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Preferences::default()),
            Err(e) => Err(err(e)),
        };
        let error = loaded.as_ref().err().cloned();
        let preferences = loaded.unwrap_or_default();
        Self {
            path,
            inner: Mutex::new(Inner {
                snapshot: Snapshot {
                    revision: 0,
                    current_version: version,
                    phase: "idle",
                    automatic: preferences.automatic,
                    last_check: preferences.last_check,
                    version: None,
                    notes: None,
                    downloaded: 0,
                    total: 0,
                    error: error.clone(),
                    install_available: install_available(),
                },
                preferences,
                preferences_error: error,
                candidate: None,
                cancel: None,
                operation: 0,
            }),
        }
    }
}
fn publish(app: &tauri::AppHandle, inner: &mut Inner) {
    inner.snapshot.revision += 1;
    let _ = app.emit("update-state", &inner.snapshot);
}
fn busy(phase: &str) -> bool {
    matches!(phase, "checking" | "downloading" | "saving" | "installing")
}
pub fn snapshot(app: &tauri::AppHandle) -> Snapshot {
    app.state::<Updates>()
        .inner
        .lock()
        .unwrap()
        .snapshot
        .clone()
}
pub fn open_history() -> Result<()> {
    let url = "https://fermatter.github.io/guide/resolved-issues/";
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("/usr/bin/open");
    #[cfg(windows)]
    let mut command = std::process::Command::new("explorer.exe");
    #[cfg(target_os = "linux")]
    let mut command = std::process::Command::new("xdg-open");
    command.arg(url).spawn().map_err(err)?;
    Ok(())
}
pub fn automatic(app: &tauri::AppHandle, enabled: bool) -> Result<()> {
    let state = app.state::<Updates>();
    let mut inner = state.inner.lock().unwrap();
    if let Some(error) = &inner.preferences_error {
        return Err(error.clone());
    }
    let mut preferences = inner.preferences.clone();
    preferences.automatic = enabled;
    json_write(&state.path, &json!(preferences))?;
    inner.preferences = preferences;
    inner.snapshot.automatic = enabled;
    publish(app, &mut inner);
    Ok(())
}
pub fn start(app: &tauri::AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let _ = check(&app, false).await;
            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
    });
}
fn due(preferences: &Preferences, time: u64) -> bool {
    preferences.automatic
        && (preferences.last_check == 0
            || time.saturating_sub(preferences.last_check) >= DAY
            || preferences.last_check > time)
}
fn client() -> Result<reqwest::Client> {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    reqwest::Client::builder()
        .user_agent("FerMatter-Updater")
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(15 * 60))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            let url = attempt.url();
            if attempt.previous().len() >= 4
                || url.scheme() != "https"
                || !url.username().is_empty()
                || url.password().is_some()
                || url.port().is_some()
                || url.host_str() != Some("release-assets.githubusercontent.com")
            {
                attempt.error("Unexpected update redirect")
            } else {
                attempt.follow()
            }
        }))
        .build()
        .map_err(err)
}
async fn bounded_get(
    url: &str,
    limit: u64,
    expected: Option<u64>,
    mut progress: impl FnMut(u64),
) -> Result<Vec<u8>> {
    let mut response = client()?
        .get(url)
        .send()
        .await
        .map_err(err)?
        .error_for_status()
        .map_err(err)?;
    if response
        .content_length()
        .is_some_and(|length| length > limit || expected.is_some_and(|expected| expected != length))
    {
        return Err("Unexpected update download size.".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(err)? {
        if bytes.len() as u64 + chunk.len() as u64 > limit {
            return Err("Update exceeds its download size limit.".into());
        }
        bytes.extend_from_slice(&chunk);
        progress(bytes.len() as u64);
    }
    if expected.is_some_and(|expected| expected != bytes.len() as u64) {
        return Err("Update download is incomplete.".into());
    }
    Ok(bytes)
}
fn target() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Some("darwin-aarch64-app"),
        ("windows", "x86_64") => Some("windows-x86_64-nsis"),
        ("linux", "x86_64") => Some("linux-x86_64-deb"),
        _ => None,
    }
}
fn install_available() -> bool {
    if cfg!(debug_assertions) || target().is_none() {
        return false;
    }
    #[cfg(target_os = "macos")]
    {
        std::env::current_exe().is_ok_and(|path| {
            path.parent().is_some_and(|p| p.ends_with("Contents/MacOS"))
                && !path.starts_with("/Volumes")
                && !path
                    .components()
                    .any(|p| p.as_os_str() == "AppTranslocation")
        })
    }
    #[cfg(not(target_os = "macos"))]
    matches!(
        tauri::utils::platform::bundle_type(),
        Some(tauri::utils::config::BundleType::Nsis | tauri::utils::config::BundleType::Deb)
    )
}
fn validate_asset(value: &Value, version: &str, target: &str) -> Result<(u64, String)> {
    let url = Url::parse(value["url"].as_str().unwrap_or("")).map_err(err)?;
    let prefix = format!("/FerMatter/fermatter.github.io/releases/download/v{version}/");
    let filename = url
        .path()
        .strip_prefix(&prefix)
        .ok_or("Update is outside the official release.")?;
    let extension = match target {
        "darwin-aarch64-app" => ".app.tar.gz",
        "windows-x86_64-nsis" => ".exe",
        "linux-x86_64-deb" => ".deb",
        _ => return Err("Unsupported update target.".into()),
    };
    if url.scheme() != "https"
        || url.host_str() != Some("github.com")
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !filename.ends_with(extension)
        || !filename
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    {
        return Err("Unexpected update asset URL.".into());
    }
    let size = value["size"]
        .as_u64()
        .filter(|s| *s > 0 && *s <= MAX_PACKAGE)
        .ok_or("Invalid update size.")?;
    let hash = value["sha256"]
        .as_str()
        .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or("Invalid update checksum.")?;
    Ok((size, hash.to_ascii_lowercase()))
}
fn release_version(value: &Value, current: &semver::Version) -> Result<Option<semver::Version>> {
    let version = semver::Version::parse(value["version"].as_str().unwrap_or("")).map_err(err)?;
    if !version.pre.is_empty() || !version.build.is_empty() {
        return Err("The stable update feed contains a preview version.".into());
    }
    if !value["platforms"].is_object() {
        return Err("The update feed has no platform map.".into());
    }
    if version <= *current {
        return Ok(None);
    }
    Ok(Some(version))
}
fn candidate(app: &tauri::AppHandle, value: Value) -> Result<Option<Candidate>> {
    let Some(version) = release_version(&value, &app.package_info().version)? else {
        return Ok(None);
    };
    let target = target().ok_or("Updates are not available for this OS and CPU.")?;
    let entry = &value["platforms"][target];
    // Platforms can be published separately; never substitute another OS/installer.
    if entry.is_null() {
        return Ok(None);
    }
    let (size, sha256) = validate_asset(entry, &version.to_string(), target)?;
    let update = updater(app, target)?.restore_update(value).map_err(err)?;
    Ok(update.map(|update| Candidate {
        update,
        size,
        sha256,
        bytes: None,
    }))
}
fn updater<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    target: &str,
) -> Result<tauri_plugin_updater::Updater> {
    app.updater_builder()
        .target(target)
        .endpoints(vec![Url::parse(FEED).map_err(err)?])
        .map_err(err)?
        .build()
        .map_err(err)
}
pub async fn check(app: &tauri::AppHandle, manual: bool) -> Result<()> {
    let state = app.state::<Updates>();
    let (operation, mut cancel) = {
        let mut inner = state.inner.lock().unwrap();
        if busy(inner.snapshot.phase) || inner.snapshot.phase == "ready" {
            return Ok(());
        }
        if !manual && (!due(&inner.preferences, now()) || inner.preferences_error.is_some()) {
            return Ok(());
        }
        let mut preferences = inner.preferences.clone();
        preferences.last_check = now();
        // Persist attempts, including failures, before contacting the service.
        if inner.preferences_error.is_none() {
            json_write(&state.path, &json!(preferences))?;
        }
        inner.preferences = preferences;
        inner.snapshot.last_check = inner.preferences.last_check;
        inner.snapshot.phase = "checking";
        inner.snapshot.error = None;
        inner.operation += 1;
        let (sender, receiver) = oneshot::channel();
        inner.cancel = Some(sender);
        publish(app, &mut inner);
        (inner.operation, receiver)
    };
    let result = tokio::select! {
        _ = &mut cancel => return Ok(()),
        result = tokio::time::timeout(Duration::from_secs(30), bounded_get(FEED, 256 * 1024, None, |_| {})) => result.map_err(err).and_then(|r| r).and_then(|bytes| serde_json::from_slice(&bytes).map_err(err)).and_then(|value| candidate(app, value)),
    };
    let mut inner = state.inner.lock().unwrap();
    if inner.operation != operation {
        return Ok(());
    }
    inner.cancel = None;
    match result {
        Ok(candidate) => {
            inner.snapshot.version = candidate.as_ref().map(|c| c.update.version.clone());
            inner.snapshot.notes = candidate.as_ref().and_then(|c| c.update.body.clone());
            inner.snapshot.phase = if candidate.is_some() {
                "available"
            } else {
                "current"
            };
            inner.snapshot.downloaded = 0;
            inner.snapshot.total = candidate.as_ref().map_or(0, |c| c.size);
            inner.candidate = candidate;
        }
        Err(error) => {
            inner.snapshot.phase = if inner.candidate.is_some() {
                "available"
            } else {
                "idle"
            };
            inner.snapshot.error = Some(format!("Could not check for updates: {error}"));
        }
    }
    publish(app, &mut inner);
    Ok(())
}
pub fn cancel(app: &tauri::AppHandle) {
    let state = app.state::<Updates>();
    let mut inner = state.inner.lock().unwrap();
    if let Some(cancel) = inner.cancel.take() {
        inner.operation += 1;
        let _ = cancel.send(());
        inner.snapshot.phase = if inner.candidate.is_some() {
            "available"
        } else {
            "idle"
        };
        inner.snapshot.downloaded = 0;
        publish(app, &mut inner);
    }
}
pub async fn download(app: &tauri::AppHandle) -> Result<()> {
    let state = app.state::<Updates>();
    let (operation, update, size, hash, mut cancel) = {
        let mut inner = state.inner.lock().unwrap();
        if busy(inner.snapshot.phase) {
            return Err("An update operation is already running.".into());
        }
        let candidate = inner.candidate.as_ref().ok_or("Check for updates first.")?;
        if candidate.bytes.is_some() {
            return Ok(());
        }
        let details = (
            candidate.update.clone(),
            candidate.size,
            candidate.sha256.clone(),
        );
        inner.operation += 1;
        let (sender, receiver) = oneshot::channel();
        inner.cancel = Some(sender);
        inner.snapshot.phase = "downloading";
        inner.snapshot.error = None;
        inner.snapshot.downloaded = 0;
        publish(app, &mut inner);
        (inner.operation, details.0, details.1, details.2, receiver)
    };
    let mut last = std::time::Instant::now();
    let result = tokio::select! {
        _ = &mut cancel => return Ok(()),
        result = bounded_get(update.download_url.as_str(), size, Some(size), |bytes| {
            if last.elapsed() < Duration::from_millis(150) && bytes != size { return; }
            last = std::time::Instant::now();
            let mut inner = state.inner.lock().unwrap();
            if inner.operation == operation {
                inner.snapshot.downloaded = bytes;
                publish(app, &mut inner);
            }
        }) => result,
    }
    .and_then(|bytes| {
        if format!("{:x}", Sha256::digest(&bytes)) != hash {
            return Err("Update checksum does not match.".into());
        }
        update.verify(&bytes).map_err(err)?;
        Ok(bytes)
    });
    let mut inner = state.inner.lock().unwrap();
    if inner.operation != operation {
        return Ok(());
    }
    inner.cancel = None;
    match result {
        Ok(bytes) => {
            inner.candidate.as_mut().unwrap().bytes = Some(bytes);
            inner.snapshot.phase = "ready";
        }
        Err(error) => {
            inner.snapshot.phase = "available";
            inner.snapshot.error = Some(format!("Update was not installed: {error}"));
        }
    }
    publish(app, &mut inner);
    Ok(())
}
pub async fn install(app: &tauri::AppHandle) -> Result<()> {
    let state = app.state::<Updates>();
    {
        let mut inner = state.inner.lock().unwrap();
        if !inner.snapshot.install_available {
            return Err("Install a packaged copy of FerMatter before updating it.".into());
        }
        if inner.snapshot.phase != "ready" {
            return Err("Download and verify the update first.".into());
        }
        inner.snapshot.phase = "saving";
        inner.snapshot.error = None;
        publish(app, &mut inner);
    }
    let token = match lifecycle::prepare_update(app).await {
        Ok(token) => token,
        Err(error) => {
            let mut inner = state.inner.lock().unwrap();
            inner.snapshot.phase = "ready";
            inner.snapshot.error = Some(format!(
                "Update cancelled; your editors remain open. {error}"
            ));
            publish(app, &mut inner);
            return Ok(());
        }
    };
    let candidate = {
        let mut inner = state.inner.lock().unwrap();
        inner.snapshot.phase = "installing";
        publish(app, &mut inner);
        inner.candidate.take().unwrap()
    };
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = tauri::process::current_binary(&handle.env())
            .map_err(err)
            .and_then(|path| install_package(&candidate).and_then(|()| restart(&path)));
        // Windows exits from the plugin after handing off to NSIS. On Unix a
        // successfully spawned replacement waits for this resident to exit.
        if result.is_ok() {
            lifecycle::exit_after_update(&handle);
            return;
        }
        let state = handle.state::<Updates>();
        let mut inner = state.inner.lock().unwrap();
        inner.snapshot.phase = "ready";
        inner.snapshot.error = result.err().map(|e| {
            format!("Could not finish the update: {e}. You can retry or use the official download.")
        });
        inner.candidate = Some(candidate);
        publish(&handle, &mut inner);
        drop(inner);
        lifecycle::cancel_update(&handle, token);
    });
    Ok(())
}
fn install_package(candidate: &Candidate) -> Result<()> {
    let bytes = candidate
        .bytes
        .as_ref()
        .ok_or("The update download is missing.")?;
    #[cfg(not(target_os = "linux"))]
    {
        candidate.update.install(bytes).map_err(err)
    }
    #[cfg(target_os = "linux")]
    {
        // Respect a cancelled OS prompt. Do not collect passwords in the app or
        // fall through to terminal sudo when no graphical polkit agent exists.
        candidate.update.verify(bytes).map_err(err)?;
        if !bytes.starts_with(b"!<arch>\n") {
            return Err("Invalid Debian package.".into());
        }
        let directory = tempfile::tempdir().map_err(err)?;
        let path = directory.path().join("fermatter.deb");
        std::fs::write(&path, bytes).map_err(err)?;
        let status = std::process::Command::new("/usr/bin/pkexec")
            .arg("/usr/bin/dpkg")
            .arg("-i")
            .arg(path)
            .status()
            .map_err(err)?;
        if !status.success() {
            return Err("Package installation was cancelled or failed. Ubuntu requires a graphical authentication agent; APT is also available.".into());
        }
        Ok(())
    }
}
fn restart(path: &Path) -> Result<()> {
    std::process::Command::new(path)
        .arg(format!("--after-update-pid={}", std::process::id()))
        .spawn()
        .map_err(err)?;
    Ok(())
}
pub fn wait_for_previous_process() -> Result<()> {
    #[cfg(unix)]
    if let Some(value) =
        std::env::args().find_map(|arg| arg.strip_prefix("--after-update-pid=").map(str::to_owned))
    {
        let pid: i32 = value.parse().map_err(err)?;
        if pid <= 1 || pid as u32 == std::process::id() {
            return Err("Invalid update parent.".into());
        }
        for _ in 0..300 {
            // Only observe the previous resident; never signal or kill it.
            if unsafe { libc::kill(pid, 0) } != 0
                && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
            {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        return Err(
            "The previous FerMatter process has not exited. Reopen the app after it closes.".into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_plugin_restores_the_manifest_and_rejects_corrupt_or_replayed_payloads() {
        use base64::Engine;
        // Independent Tauri CLI fixture, Apache-2.0/MIT; no private key.
        const PUBLIC_KEY: &str = "untrusted comment: minisign public key: A49EF4746FA6E6D7\nRWTX5qZvdPSepDGWg2bz8n6ArQnBMwgQK1geVosorcbLzcunD+KJ+cYe";
        const SIGNATURE: &str = "untrusted comment: signature from tauri secret key\nRUTX5qZvdPSepPF2zsjvP/L/TQtBLRnWRvaq+4omK1UEugtQp/USrGiI1v762sDy3id3aor7bRC9ETyeot6lMHHrWoZ5hjUM1Qc=\ntrusted comment: timestamp:1791649051\tfile:payload\tversion:2.0.0\nDDq2CLOEl5cQmC2eJRHlnZPVViOZsu1CjpPKQoei4A/XBfkDqGO0B+QMrcrHgEQ5qV3mne75tdFWhJpqUXh9AA==";
        let encode = |s: &str| base64::engine::general_purpose::STANDARD.encode(s);
        let mut context = tauri::test::mock_context(tauri::test::noop_assets());
        context.config_mut().plugins.0.insert(
            "updater".into(),
            json!({"pubkey":encode(PUBLIC_KEY),"requireSignedVersion":true}),
        );
        let app = tauri::test::mock_builder().build(context).unwrap();
        app.handle()
            .plugin(
                tauri_plugin_updater::Builder::new()
                    .pubkey(encode(PUBLIC_KEY))
                    .build(),
            )
            .unwrap();
        let updater = updater(app.handle(), "darwin-aarch64-app").unwrap();
        let mut manifest = json!({"version":"2.0.0","platforms":{"darwin-aarch64-app":{"url":"https://github.com/FerMatter/fermatter.github.io/releases/download/v2.0.0/FerMatter.app.tar.gz","signature":encode(SIGNATURE)}}});
        let update = updater.restore_update(manifest.clone()).unwrap().unwrap();
        update.verify(b"test").unwrap();
        assert!(update.verify(b"corrupt").is_err());
        manifest["version"] = json!("3.0.0");
        assert!(updater
            .restore_update(manifest)
            .unwrap()
            .unwrap()
            .verify(b"test")
            .is_err());
    }

    #[test]
    fn only_newer_stable_versions_are_eligible() {
        let current = semver::Version::new(0, 11, 0);
        for version in ["0.0.0", "0.10.9", "0.11.0"] {
            assert!(
                release_version(&json!({"version":version,"platforms":{}}), &current)
                    .unwrap()
                    .is_none()
            );
        }
        for version in ["0.12.0-rc.1", "0.11.0+preview.5", "bad", "1.0"] {
            assert!(release_version(&json!({"version":version,"platforms":{}}), &current).is_err());
        }
        assert_eq!(
            release_version(&json!({"version":"0.12.0","platforms":{}}), &current).unwrap(),
            Some(semver::Version::new(0, 12, 0))
        );
        assert!(release_version(&json!({"version":"0.12.0"}), &current).is_err());
    }
    #[tokio::test]
    async fn bounded_download_rejects_oversize_and_truncated_responses() {
        use std::io::{Read, Write};
        async fn response(
            text: &'static str,
            limit: u64,
            expected: Option<u64>,
        ) -> Result<Vec<u8>> {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let url = format!("http://{}", listener.local_addr().unwrap());
            let server = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                let _ = socket.read(&mut [0; 4096]);
                let _ = socket.write_all(text.as_bytes());
            });
            let result = bounded_get(&url, limit, expected, |_| {}).await;
            server.join().unwrap();
            result
        }
        assert_eq!(
            response(
                "HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\ntest",
                4,
                Some(4)
            )
            .await
            .unwrap(),
            b"test"
        );
        assert!(response(
            "HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\ntest",
            4,
            None
        )
        .await
        .is_err());
        assert!(response(
            "HTTP/1.1 200 OK\r\nContent-Length: 8\r\n\r\ntest",
            8,
            Some(8)
        )
        .await
        .is_err());
        assert!(response(
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nlarge\r\n0\r\n\r\n",
            4,
            None
        )
        .await
        .is_err());
        assert!(response(
            "HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\ntest",
            8,
            Some(8)
        )
        .await
        .is_err());
    }
    #[test]
    fn automatic_checks_are_opt_in_and_failed_attempts_are_throttled() {
        assert!(!due(&Preferences::default(), 100));
        let p = Preferences {
            automatic: true,
            last_check: 100,
        };
        assert!(!due(&p, 100 + DAY - 1));
        assert!(due(&p, 100 + DAY));
        assert!(due(&p, 99));
    }
    #[test]
    fn reject_other_hosts_repositories_versions_formats_and_sizes() {
        let mut entry = json!({"url":"https://github.com/FerMatter/fermatter.github.io/releases/download/v1.0.0/FerMatter.app.tar.gz", "size":10,"sha256":"a".repeat(64)});
        assert!(validate_asset(&entry, "1.0.0", "darwin-aarch64-app").is_ok());
        assert!(validate_asset(&entry, "1.0.1", "darwin-aarch64-app").is_err());
        assert!(validate_asset(&entry, "1.0.0", "windows-x86_64-nsis").is_err());
        let original = entry["url"].as_str().unwrap().to_owned();
        for url in [
            original.replace("https:", "http:"),
            original.replace("github.com", "example.com"),
            original.replace("/FerMatter/", "/other/"),
            format!("{original}?token=secret"),
            original.replace("FerMatter.app", "%2fother.app"),
        ] {
            entry["url"] = json!(url);
            assert!(validate_asset(&entry, "1.0.0", "darwin-aarch64-app").is_err());
        }
        entry["url"] = json!(original);
        for size in [0, MAX_PACKAGE + 1] {
            entry["size"] = json!(size);
            assert!(validate_asset(&entry, "1.0.0", "darwin-aarch64-app").is_err());
        }
    }
    #[test]
    fn corrupt_preferences_are_kept_and_disable_automatic_checks() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("UpdatePreferences.json");
        std::fs::write(&path, b"broken").unwrap();
        let state = Updates::new(dir.path(), "0.11.0".into());
        let inner = state.inner.lock().unwrap();
        assert!(!inner.preferences.automatic);
        assert!(inner.preferences_error.is_some());
        assert_eq!(std::fs::read(path).unwrap(), b"broken");
    }
}
