//! Private, bounded local transport. Only the resident owns storage and settings.
use super::{ApiError, ApiResult};
#[cfg(any(unix, test))]
use serde::{Deserialize, Serialize};
#[cfg(any(unix, test))]
use serde_json::json;
use serde_json::Value;
#[cfg(any(unix, test))]
use std::io::{Read, Write};
use std::sync::{atomic::AtomicBool, Arc};
#[cfg(unix)]
use std::{sync::atomic::Ordering, time::Duration};

#[cfg(any(unix, test))]
const REQUEST_LIMIT: usize = 1024 * 1024;
#[cfg(unix)]
const RESPONSE_LIMIT: usize = 16 * 1024 * 1024;
#[cfg(any(unix, test))]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    api_version: u32,
    operation: String,
    arguments: Value,
}
#[cfg(any(unix, test))]
fn io_error(e: impl std::fmt::Display) -> ApiError {
    ApiError::new("connection_error", e.to_string())
}
#[cfg(any(unix, test))]
fn read_frame<T: serde::de::DeserializeOwned>(
    stream: &mut impl Read,
    limit: usize,
) -> Result<T, ApiError> {
    let mut bytes = [0; 4];
    stream.read_exact(&mut bytes).map_err(io_error)?;
    let length = u32::from_be_bytes(bytes) as usize;
    if length > limit {
        return Err(ApiError::new(
            "message_too_large",
            "Local API message exceeds its size limit.",
        ));
    }
    let mut bytes = vec![0; length];
    stream.read_exact(&mut bytes).map_err(io_error)?;
    serde_json::from_slice(&bytes).map_err(|e| ApiError::new("invalid_request", e.to_string()))
}
#[cfg(any(unix, test))]
fn write_frame(
    stream: &mut impl Write,
    value: &impl Serialize,
    limit: usize,
) -> Result<(), ApiError> {
    let bytes = serde_json::to_vec(value).map_err(io_error)?;
    if bytes.len() > limit {
        return Err(ApiError::new(
            "message_too_large",
            "Local API message exceeds its size limit.",
        ));
    }
    stream
        .write_all(&(bytes.len() as u32).to_be_bytes())
        .map_err(io_error)?;
    stream.write_all(&bytes).map_err(io_error)
}

#[cfg(unix)]
mod local {
    use super::*;
    use crate::{compiler, files, Runtime};
    use std::{
        fs,
        net::Shutdown,
        os::unix::{
            fs::{MetadataExt, PermissionsExt},
            net::{UnixListener, UnixStream},
        },
        path::PathBuf,
        process::{Command, Stdio},
        sync::atomic::AtomicUsize,
        time::Instant,
    };
    use tauri::{Emitter, Manager};
    pub struct Server {
        path: PathBuf,
        stop: Arc<AtomicBool>,
    }
    impl Drop for Server {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            let _ = fs::remove_file(&self.path);
        }
    }
    pub fn start(app: &tauri::AppHandle) -> files::Result<Server> {
        let path = crate::single_instance::automation_path(&app.config().identifier);
        if path.exists() {
            fs::remove_file(&path).map_err(files::err)?;
        }
        let listener = UnixListener::bind(&path).map_err(files::err)?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).map_err(files::err)?;
        listener.set_nonblocking(true).map_err(files::err)?;
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let app = app.clone();
        std::thread::spawn(move || {
            let active = Arc::new(AtomicUsize::new(0));
            while !stopped.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        if active.load(Ordering::SeqCst) >= 16 {
                            let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
                            let _ = write_frame(
                                &mut stream,
                                &json!({"error":{"code":"busy","message":"Too many local API requests. Retry later."}}),
                                RESPONSE_LIMIT,
                            );
                            continue;
                        }
                        active.fetch_add(1, Ordering::SeqCst);
                        let count = active.clone();
                        let app = app.clone();
                        std::thread::spawn(move || {
                            handle(&app, &mut stream);
                            count.fetch_sub(1, Ordering::SeqCst);
                        });
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(40))
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Server { path, stop })
    }
    fn configure(stream: &UnixStream) -> std::io::Result<()> {
        // BSD accept() inherits O_NONBLOCK from the listening socket.
        stream.set_nonblocking(false)?;
        stream.set_read_timeout(Some(Duration::from_secs(3)))?;
        stream.set_write_timeout(Some(Duration::from_secs(3)))
    }
    fn handle(app: &tauri::AppHandle, stream: &mut UnixStream) {
        if configure(stream).is_err() {
            return;
        }
        let result = (|| -> ApiResult {
            let request: Request = read_frame(stream, REQUEST_LIMIT)?;
            if request.api_version != 1 {
                return Err(ApiError::new(
                    "unsupported_version",
                    "This resident supports API version 1.",
                ));
            }
            if crate::lifecycle::quitting(app) {
                return Err(ApiError::new("busy", "The app is saving and quitting."));
            }
            let runtime = app.state::<Arc<Runtime>>();
            let cancel = compiler::Cancellation::with_tools(runtime.tools.lock().unwrap().clone());
            let mut monitor = stream.try_clone().map_err(io_error)?;
            let cancellation = cancel.clone();
            let monitor_thread = std::thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(300);
                // No more request data is expected. EOF/cancellation or a deadline stops compilers.
                while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
                    let _ = monitor.set_read_timeout(Some(remaining));
                    match monitor.read(&mut [0]) {
                        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                        _ => break,
                    }
                }
                cancellation.cancel();
            });
            let result = if request.operation == "open_in_app" {
                (|| {
                    let args: super::super::schema::Open = super::super::parse(request.arguments)?;
                    if !args.path.is_absolute() {
                        return Err(ApiError::new("invalid_arguments", "path must be absolute."));
                    }
                    let source = crate::roundtrip::read_svg(&args.path)?;
                    crate::resident::import_svg(app, &[source])?;
                    Ok(json!({"apiVersion":1,"status":"queued"}))
                })()
            } else {
                let service = super::super::Service {
                    store: &runtime.store,
                    settings: &runtime.settings,
                };
                service.call(
                    &request.operation,
                    request.arguments,
                    &cancel,
                    &mut |progress| {
                        if write_frame(stream, &json!({"progress":progress}), RESPONSE_LIMIT)
                            .is_err()
                        {
                            cancel.cancel();
                        }
                    },
                )
            };
            let _ = stream.shutdown(Shutdown::Read);
            let _ = monitor_thread.join();
            if request.operation == "create_variant"
                && result.as_ref().is_ok_and(|v| v["created"] == true)
            {
                // Send the full saved snapshot so out-of-order events include every earlier addition.
                if let Ok(project) = runtime
                    .store
                    .load(result.as_ref().unwrap()["projectId"].as_str().unwrap_or(""))
                {
                    let _ = app.emit_to("main", "automation-project-changed", &project);
                }
            }
            result
        })();
        let frame = match result {
            Ok(value) => json!({"result":value}),
            Err(error) => json!({"error":error}),
        };
        if let Err(error) = write_frame(stream, &frame, RESPONSE_LIMIT) {
            let _ = write_frame(stream, &json!({"error":error}), RESPONSE_LIMIT);
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn accepted_connection_waits_for_data_until_peer_disconnects() {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("socket");
            let listener = UnixListener::bind(&path).unwrap();
            listener.set_nonblocking(true).unwrap();
            let client = UnixStream::connect(&path).unwrap();
            let (mut server, _) = listener.accept().unwrap();
            configure(&server).unwrap();
            let (tx, rx) = std::sync::mpsc::channel();
            let worker = std::thread::spawn(move || {
                let _ = tx.send(server.read(&mut [0]));
            });
            assert!(rx.recv_timeout(Duration::from_millis(100)).is_err());
            drop(client);
            assert_eq!(rx.recv_timeout(Duration::from_secs(1)).unwrap().unwrap(), 0);
            worker.join().unwrap();
        }
    }
    fn connect(identity: &str, cancel: &AtomicBool) -> Result<UnixStream, ApiError> {
        let socket = crate::single_instance::automation_path(identity);
        let mut started = false;
        let mut existing = false;
        let deadline = Instant::now() + Duration::from_secs(12);
        loop {
            if cancel.load(Ordering::SeqCst) {
                return Err(ApiError::new("cancelled", "Cancelled"));
            }
            if let Ok(metadata) = fs::symlink_metadata(socket.parent().unwrap()) {
                if !metadata.is_dir()
                    || metadata.uid() != unsafe { libc::geteuid() }
                    || metadata.mode() & 0o077 != 0
                {
                    return Err(ApiError::new(
                        "connection_error",
                        "Unsafe resident coordination directory.",
                    ));
                }
            }
            if let Ok(stream) = UnixStream::connect(&socket) {
                return Ok(stream);
            }
            if !started {
                // An older live resident cannot serve this API; don't create a second instance.
                existing = UnixStream::connect(socket.with_file_name("ipc")).is_ok();
                if !existing {
                    let mut child = Command::new(std::env::current_exe().map_err(io_error)?)
                        .arg("--automation-background")
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                        .map_err(io_error)?;
                    std::thread::spawn(move || {
                        let _ = child.wait();
                    });
                }
                started = true;
            }
            if Instant::now() >= deadline {
                if existing {
                    return Err(ApiError::new(
                        "restart_required",
                        "Restart FerMatter using the updated app to enable its local API.",
                    ));
                }
                return Err(ApiError::new(
                    "connection_error",
                    "FerMatter did not start its local API. Open the app and try again.",
                ));
            }
            std::thread::sleep(Duration::from_millis(60));
        }
    }
    pub fn call(
        identity: &str,
        operation: &str,
        arguments: Value,
        cancel: Arc<AtomicBool>,
        progress: &mut impl FnMut(Value),
    ) -> ApiResult {
        let mut stream = connect(identity, &cancel)?;
        stream
            .set_write_timeout(Some(Duration::from_secs(3)))
            .map_err(io_error)?;
        stream
            .set_read_timeout(Some(Duration::from_secs(310)))
            .map_err(io_error)?;
        write_frame(
            &mut stream,
            &Request {
                api_version: 1,
                operation: operation.into(),
                arguments,
            },
            REQUEST_LIMIT,
        )?;
        let watcher = stream.try_clone().map_err(io_error)?;
        let finished = Arc::new(AtomicBool::new(false));
        let done = finished.clone();
        let cancelled = cancel.clone();
        let monitor = std::thread::spawn(move || {
            while !done.load(Ordering::SeqCst) {
                if cancelled.load(Ordering::SeqCst) {
                    let _ = watcher.shutdown(Shutdown::Both);
                    break;
                }
                std::thread::sleep(Duration::from_millis(30));
            }
        });
        let result = (|| loop {
            let value: Value = read_frame(&mut stream, RESPONSE_LIMIT)?;
            if let Some(error) = value.get("error") {
                return Err(serde_json::from_value(error.clone()).map_err(io_error)?);
            }
            if let Some(value) = value.get("result") {
                return Ok(value.clone());
            }
            if let Some(value) = value.get("progress") {
                progress(value.clone());
            }
        })();
        finished.store(true, Ordering::SeqCst);
        let _ = monitor.join();
        if cancel.load(Ordering::SeqCst) {
            Err(ApiError::new("cancelled", "Cancelled"))
        } else {
            result
        }
    }
}
#[cfg(unix)]
pub use local::{call, start};
#[cfg(not(unix))]
pub fn call(
    _identity: &str,
    _operation: &str,
    _arguments: Value,
    _cancel: Arc<AtomicBool>,
    _progress: &mut impl FnMut(Value),
) -> ApiResult {
    Err(ApiError::new(
        "unsupported_platform",
        "Local automation transport is currently available on macOS/Unix.",
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oversized_request_rejected_before_allocation() {
        let bytes = ((REQUEST_LIMIT + 1) as u32).to_be_bytes();
        assert_eq!(
            read_frame::<Value>(&mut bytes.as_slice(), REQUEST_LIMIT)
                .unwrap_err()
                .code,
            "message_too_large"
        );
    }
    #[test]
    fn protocol_requires_version_and_object_shape() {
        let mut bytes = vec![];
        write_frame(
            &mut bytes,
            &json!({"apiVersion":1,"operation":"list_projects","arguments":{}}),
            REQUEST_LIMIT,
        )
        .unwrap();
        let r: Request = read_frame(&mut bytes.as_slice(), REQUEST_LIMIT).unwrap();
        assert_eq!(r.api_version, 1);
        assert!(serde_json::from_value::<Request>(
            json!({"operation":"list_projects","arguments":{}})
        )
        .is_err());
    }
}
