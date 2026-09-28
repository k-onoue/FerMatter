//! One resident per user and bundle identity, even when copies live at different paths.
//! Claim the lock synchronously before creating any window, tray or global shortcut.
#[cfg(unix)]
mod unix {
    use crate::files::{err, Result};
    use serde::{Deserialize, Serialize};
    use std::{
        fs::{self, File, OpenOptions},
        io::{Read, Write},
        os::{
            fd::AsRawFd,
            unix::{
                fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
                net::{UnixListener, UnixStream},
            },
        },
        path::{Path, PathBuf},
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        time::{Duration, Instant},
    };

    const LIMIT: usize = 64 * 1024;
    #[derive(Debug, Default, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Launch {
        paths: Vec<PathBuf>,
        activate: bool,
    }
    impl Launch {
        fn validate(&self) -> Result<()> {
            if self.paths.len() > 16 {
                return Err("Open up to 16 SVG files at once.".into());
            }
            if self.paths.iter().any(|p| {
                !p.is_absolute() || !p.extension().is_some_and(|s| s.eq_ignore_ascii_case("svg"))
            }) {
                return Err("Only absolute SVG file paths may be forwarded.".into());
            }
            Ok(())
        }
    }
    pub struct Guard {
        _lock: File,
        listener: UnixListener,
        socket: PathBuf,
        stop: Arc<AtomicBool>,
    }
    pub enum Claim {
        Primary(Guard),
        Secondary(PathBuf),
    }
    impl Drop for Guard {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            let _ = fs::remove_file(&self.socket);
            // Never remove the lock inode: other launches may already hold its fd.
        }
    }
    pub fn claim(identity: &str) -> Result<Claim> {
        let uid = unsafe { libc::geteuid() };
        let directory =
            std::env::temp_dir().join(format!("mathlidelux-{uid}-{}", crate::files::id(identity)));
        claim_at(&directory)
    }
    fn claim_at(directory: &Path) -> Result<Claim> {
        let uid = unsafe { libc::geteuid() };
        match fs::DirBuilder::new().mode(0o700).create(directory) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(err(e)),
        }
        let metadata = fs::symlink_metadata(directory).map_err(err)?;
        if !metadata.is_dir() || metadata.uid() != uid || metadata.mode() & 0o077 != 0 {
            return Err("Unsafe resident coordination directory.".into());
        }
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(directory.join("lock"))
            .map_err(err)?;
        let metadata = lock.metadata().map_err(err)?;
        if !metadata.is_file() || metadata.uid() != uid || metadata.mode() & 0o077 != 0 {
            return Err("Unsafe resident coordination lock.".into());
        }
        let socket = directory.join("ipc");
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::WouldBlock {
                return Ok(Claim::Secondary(socket));
            }
            return Err(err(error));
        }
        // Only the process holding the lock can clean up an abandoned socket.
        match fs::remove_file(&socket) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(err(e)),
        }
        let listener = UnixListener::bind(&socket).map_err(err)?;
        fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).map_err(err)?;
        listener.set_nonblocking(true).map_err(err)?;
        Ok(Claim::Primary(Guard {
            _lock: lock,
            listener,
            socket,
            stop: Arc::new(AtomicBool::new(false)),
        }))
    }
    fn read_launch(stream: &mut UnixStream) -> Result<Launch> {
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .map_err(err)?;
        let mut length = [0; 4];
        stream.read_exact(&mut length).map_err(err)?;
        let length = u32::from_be_bytes(length) as usize;
        if length > LIMIT {
            return Err("Resident launch request is too large.".into());
        }
        let mut bytes = vec![0; length];
        stream.read_exact(&mut bytes).map_err(err)?;
        let launch: Launch = serde_json::from_slice(&bytes).map_err(err)?;
        launch.validate()?;
        Ok(launch)
    }
    fn send(socket: &Path, launch: &Launch) -> Result<()> {
        launch.validate()?;
        let bytes = serde_json::to_vec(launch).map_err(err)?;
        if bytes.len() > LIMIT {
            return Err("Resident launch request is too large.".into());
        }
        let deadline = Instant::now() + Duration::from_secs(8);
        let mut stream = loop {
            match UnixStream::connect(socket) {
                Ok(stream) => break stream,
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
                    ) && Instant::now() < deadline =>
                {
                    std::thread::sleep(Duration::from_millis(40))
                }
                Err(e) => return Err(err(e)),
            }
        };
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .map_err(err)?;
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .map_err(err)?;
        stream
            .write_all(&(bytes.len() as u32).to_be_bytes())
            .map_err(err)?;
        stream.write_all(&bytes).map_err(err)?;
        let mut ack = [0];
        stream.read_exact(&mut ack).map_err(err)?;
        if ack[0] != 1 {
            return Err("The running Mathlidelux could not accept this launch.".into());
        }
        Ok(())
    }
    impl Guard {
        pub fn listen(&self, app: &tauri::AppHandle) -> Result<()> {
            let listener = self.listener.try_clone().map_err(err)?;
            let stop = self.stop.clone();
            let app = app.clone();
            std::thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            let result = read_launch(&mut stream).and_then(|launch| {
                                if crate::lifecycle::quitting(&app) {
                                    return Err("Mathlidelux is saving and quitting.".into());
                                }
                                if !launch.paths.is_empty() {
                                    crate::resident::open_files(&app, launch.paths);
                                } else if launch.activate {
                                    crate::resident::show_projects(&app);
                                }
                                Ok(())
                            });
                            let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
                            let _ = stream.write_all(&[u8::from(result.is_ok())]);
                            if let Err(error) = result {
                                eprintln!("Resident launch rejected: {error}");
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(40))
                        }
                        Err(e) => {
                            eprintln!("Resident listener stopped: {e}");
                            break;
                        }
                    }
                }
            });
            Ok(())
        }
    }
    fn arguments() -> Vec<PathBuf> {
        let cwd = std::env::current_dir().unwrap_or_default();
        std::env::args_os()
            .skip(1)
            .map(PathBuf::from)
            .filter(|p| p.extension().is_some_and(|s| s.eq_ignore_ascii_case("svg")))
            .map(|p| if p.is_absolute() { p } else { cwd.join(p) })
            .collect()
    }
    pub fn primary_arguments(app: &tauri::AppHandle) {
        let paths = arguments();
        if !paths.is_empty() {
            crate::resident::open_files(app, paths);
        }
    }
    pub fn secondary(mut context: tauri::Context<tauri::Wry>, socket: PathBuf) {
        // LaunchServices can deliver Open Documents after setup. Keep a short,
        // windowless event loop to forward those events before exiting; never
        // create another resident, tray, shortcut registrar or storage runtime.
        context.config_mut().app.windows.clear();
        let paths = arguments();
        let callback_socket = socket.clone();
        let sent = Arc::new(AtomicBool::new(false));
        let sent_open = sent.clone();
        let app = tauri::Builder::default()
            .setup(|app| {
                #[cfg(target_os = "macos")]
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
                Ok(())
            })
            .build(context)
            .expect("Unable to forward Mathlidelux launch");
        app.run(move |app, event| match event {
            tauri::RunEvent::Ready => {
                let app = app.clone();
                let socket = socket.clone();
                let paths = paths.clone();
                let sent = sent.clone();
                std::thread::spawn(move || {
                    if !paths.is_empty() {
                        if let Err(error) = send(
                            &socket,
                            &Launch {
                                paths,
                                activate: false,
                            },
                        ) {
                            eprintln!("Could not forward SVG: {error}");
                        }
                        sent.store(true, Ordering::SeqCst);
                    }
                    std::thread::sleep(Duration::from_secs(2));
                    if !sent.load(Ordering::SeqCst) {
                        if let Err(error) = send(
                            &socket,
                            &Launch {
                                paths: vec![],
                                activate: true,
                            },
                        ) {
                            eprintln!("Could not activate Mathlidelux: {error}");
                        }
                    }
                    app.exit(0);
                });
            }
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Opened { urls } => {
                sent_open.store(true, Ordering::SeqCst);
                let paths = urls
                    .into_iter()
                    .filter_map(|u| u.to_file_path().ok())
                    .collect();
                if let Err(error) = send(
                    &callback_socket,
                    &Launch {
                        paths,
                        activate: false,
                    },
                ) {
                    eprintln!("Could not forward SVG: {error}");
                }
            }
            _ => {}
        });
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn one_identity_has_one_owner_and_recovers_after_owner_exits() {
            let parent = tempfile::tempdir().unwrap();
            let directory = parent.path().join("resident");
            let Claim::Primary(first) = claim_at(&directory).unwrap() else {
                panic!()
            };
            assert!(matches!(claim_at(&directory).unwrap(), Claim::Secondary(_)));
            drop(first);
            assert!(matches!(claim_at(&directory).unwrap(), Claim::Primary(_)));
        }
        #[test]
        fn concurrent_claims_cannot_both_create_residents() {
            let parent = tempfile::tempdir().unwrap();
            let directory = parent.path().join("resident");
            let barrier = Arc::new(std::sync::Barrier::new(8));
            let threads: Vec<_> = (0..8)
                .map(|_| {
                    let path = directory.clone();
                    let barrier = barrier.clone();
                    std::thread::spawn(move || {
                        barrier.wait();
                        claim_at(&path).unwrap()
                    })
                })
                .collect();
            let claims: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
            assert_eq!(
                claims
                    .iter()
                    .filter(|c| matches!(c, Claim::Primary(_)))
                    .count(),
                1
            );
        }
        #[test]
        fn bounded_protocol_rejects_invalid_paths_and_forwards_batches() {
            assert!(Launch {
                paths: vec![PathBuf::from("relative.svg")],
                activate: false
            }
            .validate()
            .is_err());
            assert!(Launch {
                paths: vec![PathBuf::from("/tmp/x.svg"); 17],
                activate: false
            }
            .validate()
            .is_err());
            let (mut writer, mut reader) = UnixStream::pair().unwrap();
            writer
                .write_all(&((LIMIT + 1) as u32).to_be_bytes())
                .unwrap();
            assert!(read_launch(&mut reader).is_err());
            let parent = tempfile::tempdir().unwrap();
            let socket = parent.path().join("ipc");
            let listener = UnixListener::bind(&socket).unwrap();
            let task = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                let value = read_launch(&mut stream).unwrap();
                stream.write_all(&[1]).unwrap();
                value
            });
            send(
                &socket,
                &Launch {
                    paths: vec!["/tmp/test.svg".into()],
                    activate: false,
                },
            )
            .unwrap();
            assert_eq!(
                task.join().unwrap().paths,
                vec![PathBuf::from("/tmp/test.svg")]
            );
        }
        #[test]
        fn refuses_symlinked_coordination_directory_and_lock() {
            let parent = tempfile::tempdir().unwrap();
            let directory = parent.path().join("resident");
            std::os::unix::fs::symlink(parent.path(), &directory).unwrap();
            assert!(claim_at(&directory).is_err());
            fs::remove_file(&directory).unwrap();
            fs::create_dir(&directory).unwrap();
            fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
            std::os::unix::fs::symlink(parent.path().join("untouched"), directory.join("lock"))
                .unwrap();
            assert!(claim_at(&directory).is_err());
            assert!(!parent.path().join("untouched").exists());
        }
    }
}
#[cfg(unix)]
pub use unix::*;
