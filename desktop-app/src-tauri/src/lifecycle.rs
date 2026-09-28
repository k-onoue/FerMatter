use crate::files::{err, Result};
use serde_json::json;
use std::collections::HashSet;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tauri::{Emitter, Manager};

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
    if pending
        .as_ref()
        .is_some_and(|request| !request.quit && request.waiting.contains(label))
    {
        pending.take();
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
    for (_, window) in windows {
        if window.emit("flush-window", json!({"token":token})).is_err() {
            state.pending.lock().unwrap().take();
            return;
        }
    }
    // A missing/unresponsive WebView must never cause data to be discarded.
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(std::time::Duration::from_secs(30));
        let state = app.state::<Lifecycle>();
        let mut pending = state.pending.lock().unwrap();
        if pending
            .as_ref()
            .is_some_and(|request| request.token == token)
        {
            pending.take();
            let _ = app.emit(
                "persistence-error",
                "Closing was cancelled because an editor did not finish saving. Please try again.",
            );
        }
    });
}
pub fn complete(app: &tauri::AppHandle, label: &str, token: u64, error: &str) -> Result<()> {
    let state = app.state::<Lifecycle>();
    let quit = match state.acknowledge(label, token, error.is_empty()) {
        FlushOutcome::Pending => return Ok(()),
        FlushOutcome::Failed => {
            if let Some(window) = app.get_webview_window(label) {
                let _ = window.show();
                let _ = window.set_focus();
            }
            return Err(error.into());
        }
        FlushOutcome::Complete(quit) => quit,
    };
    if quit {
        state.exiting.store(true, Ordering::SeqCst);
        app.exit(0);
    } else if let Some(window) = app.get_webview_window(label) {
        if label == "main" {
            window.hide().map_err(err)?;
        } else {
            window.destroy().map_err(err)?;
        }
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
        assert_eq!(
            state.acknowledge("instant", 2, true),
            FlushOutcome::Complete(true)
        );
    }
    #[test]
    fn one_failed_save_cancels_quit_and_late_success_cannot_close_any_window() {
        let state = request(7);
        assert_eq!(state.acknowledge("instant", 7, false), FlushOutcome::Failed);
        assert_eq!(state.acknowledge("main", 7, true), FlushOutcome::Pending);
        assert!(state.pending.lock().unwrap().is_none());
    }
}
