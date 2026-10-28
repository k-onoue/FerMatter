use crate::files::{err, Result};
use serde_json::json;
use std::collections::HashSet;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tauri::{Emitter, Manager};

// Wry executes tasks inline when already on its main thread. Native window
// admission and close completion therefore share one ordering without a lock
// held while a window is created or destroyed.
pub fn on_main<T: Send + 'static>(
    app: &tauri::AppHandle,
    operation: impl FnOnce(tauri::AppHandle) -> Result<T> + Send + 'static,
) -> Result<T> {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let _ = sender.send(operation(handle));
    })
    .map_err(err)?;
    receiver.recv().map_err(err)?
}

#[derive(Default)]
pub struct Lifecycle {
    pending: Mutex<Option<Flush>>,
    sequence: Mutex<u64>,
    exiting: AtomicBool,
}
struct Flush {
    token: u64,
    waiting: HashSet<String>,
    quit: bool,
}
#[derive(Debug, PartialEq)]
enum FlushOutcome {
    Pending,
    Failed,
    Complete(bool),
}
impl Lifecycle {
    fn acknowledge(&self, label: &str, token: u64, success: bool) -> FlushOutcome {
        let mut pending = self.pending.lock().unwrap();
        let Some(request) = pending.as_mut().filter(|request| request.token == token) else {
            return FlushOutcome::Pending;
        };
        if !request.waiting.contains(label) {
            return FlushOutcome::Pending;
        }
        if !success {
            pending.take();
            return FlushOutcome::Failed;
        }
        request.waiting.remove(label);
        if !request.waiting.is_empty() {
            return FlushOutcome::Pending;
        }
        let quit = request.quit;
        if quit {
            // Keep admission closed between the last acknowledgement and app.exit.
            self.exiting.store(true, Ordering::SeqCst);
        }
        pending.take();
        FlushOutcome::Complete(quit)
    }
}
pub fn quitting(app: &tauri::AppHandle) -> bool {
    let state = app.state::<Lifecycle>();
    state.exiting.load(Ordering::SeqCst)
        || state
            .pending
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|request| request.quit)
}
pub fn exiting(app: &tauri::AppHandle) -> bool {
    app.state::<Lifecycle>().exiting.load(Ordering::SeqCst)
}
// Showing the main window again supersedes an unfinished hide request.
pub fn cancel_close(app: &tauri::AppHandle, label: &str) {
    let state = app.state::<Lifecycle>();
    let mut pending = state.pending.lock().unwrap();
    let cancelled = if pending
        .as_ref()
        .is_some_and(|request| !request.quit && request.waiting.contains(label))
    {
        pending.take().map(|request| request.token)
    } else {
        None
    };
    drop(pending);
    if let Some(token) = cancelled {
        resume_windows(app, token);
    }
}
// Acknowledging one window is not permission to edit while another window saves.
// Only the native request's final outcome releases its frontend interaction leases.
fn resume_windows(app: &tauri::AppHandle, token: u64) {
    let _ = app.emit("flush-window-finished", json!({"token":token}));
}
fn cancel_token(state: &Lifecycle, token: u64) -> bool {
    let mut pending = state.pending.lock().unwrap();
    if pending
        .as_ref()
        .is_some_and(|request| request.token == token)
    {
        pending.take();
        true
    } else {
        false
    }
}
pub fn request_quit(app: &tauri::AppHandle) {
    begin(app, None);
}
pub fn request_close(app: &tauri::AppHandle, label: &str) {
    begin(app, Some(label));
}
fn begin(app: &tauri::AppHandle, label: Option<&str>) {
    let state = app.state::<Lifecycle>();
    let mut pending = state.pending.lock().unwrap();
    // Explicit Quit supersedes an unfinished single-window hide/close.
    if pending
        .as_ref()
        .is_some_and(|request| request.quit || label.is_some())
    {
        return;
    }
    let superseded = pending.take().map(|request| request.token);
    let windows: Vec<_> = app
        .webview_windows()
        .into_iter()
        .filter(|(name, _)| label.is_none_or(|label| label == name))
        .collect();
    let mut sequence = state.sequence.lock().unwrap();
    *sequence += 1;
    let token = *sequence;
    *pending = Some(Flush {
        token,
        waiting: windows.iter().map(|(name, _)| name.clone()).collect(),
        quit: label.is_none(),
    });
    drop(pending);
    if let Some(token) = superseded {
        resume_windows(app, token);
    }
    for (_, window) in windows {
        if window.emit("flush-window", json!({"token":token})).is_err() {
            if cancel_token(&state, token) {
                resume_windows(app, token);
            }
            return;
        }
    }
    // A missing/unresponsive WebView must never cause data to be discarded.
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(std::time::Duration::from_secs(30));
        let state = app.state::<Lifecycle>();
        if cancel_token(&state, token) {
            resume_windows(&app, token);
            let _ = app.emit(
                "persistence-error",
                "Closing was cancelled because an editor did not finish saving. Please try again.",
            );
        }
    });
}
pub fn complete(app: &tauri::AppHandle, label: &str, token: u64, error: &str) -> Result<()> {
    let label = label.to_owned();
    let error = error.to_owned();
    on_main(app, move |app| {
        complete_on_main(&app, &label, token, &error)
    })
}
fn complete_on_main(app: &tauri::AppHandle, label: &str, token: u64, error: &str) -> Result<()> {
    let state = app.state::<Lifecycle>();
    let quit = match state.acknowledge(label, token, error.is_empty()) {
        FlushOutcome::Pending => return Ok(()),
        FlushOutcome::Failed => {
            resume_windows(app, token);
            if let Some(window) = app.get_webview_window(label) {
                let _ = window.show();
                let _ = window.set_focus();
            }
            return Err(error.into());
        }
        FlushOutcome::Complete(quit) => quit,
    };
    if quit {
        // Every window remains inert until process exit.
        app.exit(0);
    } else if let Some(window) = app.get_webview_window(label) {
        let result = if label == "main" {
            window.hide()
        } else {
            #[cfg(windows)]
            {
                // DestroyWindow also destroys its attached HMENU. Tauri shares
                // the app menu with main/other Instant windows, so detach this
                // window first to keep their menus and theme state valid.
                window.remove_menu().and_then(|_| window.destroy())
            }
            #[cfg(not(windows))]
            {
                window.destroy()
            }
        };
        resume_windows(app, token);
        result.map_err(err)?;
    } else {
        resume_windows(app, token);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(token: u64) -> Lifecycle {
        Lifecycle {
            pending: Mutex::new(Some(Flush {
                token,
                waiting: ["main".into(), "instant".into()].into(),
                quit: true,
            })),
            ..Default::default()
        }
    }
    #[test]
    fn quit_waits_for_every_window_and_ignores_duplicate_or_stale_acknowledgements() {
        let state = request(2);
        assert_eq!(state.acknowledge("main", 1, true), FlushOutcome::Pending);
        assert_eq!(state.acknowledge("other", 2, true), FlushOutcome::Pending);
        assert_eq!(state.acknowledge("main", 2, true), FlushOutcome::Pending);
        assert_eq!(state.acknowledge("main", 2, true), FlushOutcome::Pending);
        assert!(!state.exiting.load(Ordering::SeqCst));
        assert_eq!(
            state.acknowledge("instant", 2, true),
            FlushOutcome::Complete(true)
        );
        assert!(state.exiting.load(Ordering::SeqCst));
    }
    #[test]
    fn one_failed_save_cancels_quit_and_late_success_cannot_close_any_window() {
        let state = request(7);
        assert_eq!(state.acknowledge("instant", 7, false), FlushOutcome::Failed);
        assert_eq!(state.acknowledge("main", 7, true), FlushOutcome::Pending);
        assert!(state.pending.lock().unwrap().is_none());
        assert!(!state.exiting.load(Ordering::SeqCst));
    }
    #[test]
    fn stale_timeout_cannot_cancel_a_newer_request() {
        let state = request(8);
        assert!(!cancel_token(&state, 7));
        assert!(state.pending.lock().unwrap().is_some());
        assert!(cancel_token(&state, 8));
        assert!(state.pending.lock().unwrap().is_none());
    }
}
