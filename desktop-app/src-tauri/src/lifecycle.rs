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
    action: Action,
    completion: Option<tokio::sync::oneshot::Sender<Result<u64>>>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum Action {
    Close,
    Quit,
    Update,
}
#[derive(Debug, PartialEq)]
enum FlushOutcome {
    Pending,
    Failed,
    Complete(Action),
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
            if let Some(sender) = request.completion.take() {
                let _ = sender.send(Err("An editor could not save.".into()));
            }
            pending.take();
            return FlushOutcome::Failed;
        }
        request.waiting.remove(label);
        if !request.waiting.is_empty() {
            return FlushOutcome::Pending;
        }
        let action = request.action;
        if action == Action::Update {
            if let Some(sender) = request.completion.take() {
                let _ = sender.send(Ok(token));
            }
            return FlushOutcome::Complete(action);
        }
        if action == Action::Quit {
            // Keep admission closed between the last acknowledgement and app.exit.
            self.exiting.store(true, Ordering::SeqCst);
        }
        pending.take();
        FlushOutcome::Complete(action)
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
            .is_some_and(|request| request.action != Action::Close)
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
        .is_some_and(|request| request.action == Action::Close && request.waiting.contains(label))
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
    cancel_token_if(state, token, false)
}
fn cancel_token_if(state: &Lifecycle, token: u64, only_waiting: bool) -> bool {
    let mut pending = state.pending.lock().unwrap();
    if pending.as_ref().is_some_and(|request| {
        request.token == token && (!only_waiting || !request.waiting.is_empty())
    }) {
        if let Some(mut request) = pending.take() {
            if let Some(sender) = request.completion.take() {
                let _ = sender.send(Err("An editor did not finish saving.".into()));
            }
        }
        true
    } else {
        false
    }
}
pub fn request_quit(app: &tauri::AppHandle) {
    let _ = begin(app, None, Action::Quit, None);
}
pub fn request_close(app: &tauri::AppHandle, label: &str) {
    let _ = begin(app, Some(label), Action::Close, None);
}
fn begin(
    app: &tauri::AppHandle,
    label: Option<&str>,
    action: Action,
    completion: Option<tokio::sync::oneshot::Sender<Result<u64>>>,
) -> Result<()> {
    let state = app.state::<Lifecycle>();
    let mut pending = state.pending.lock().unwrap();
    // Explicit Quit supersedes an unfinished single-window hide/close.
    if pending
        .as_ref()
        .is_some_and(|request| request.action != Action::Close || label.is_some())
    {
        return Err("A window is already closing or an update is being installed.".into());
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
        action,
        completion,
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
            return Err("Could not ask an editor to save.".into());
        }
    }
    // A missing/unresponsive WebView must never cause data to be discarded.
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(std::time::Duration::from_secs(30));
        let state = app.state::<Lifecycle>();
        // The check and cancellation share the same lock: a final save arriving
        // at the deadline must not resume editors after granting install permission.
        if cancel_token_if(&state, token, true) {
            resume_windows(&app, token);
            let _ = app.emit(
                "persistence-error",
                "Closing was cancelled because an editor did not finish saving. Please try again.",
            );
        }
    });
    Ok(())
}
pub async fn prepare_update(app: &tauri::AppHandle) -> Result<u64> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    on_main(app, move |app| {
        begin(&app, None, Action::Update, Some(sender))
    })?;
    receiver.await.map_err(err)?
}
pub fn cancel_update(app: &tauri::AppHandle, token: u64) {
    let _ = on_main(app, move |app| {
        if cancel_token(&app.state::<Lifecycle>(), token) {
            resume_windows(&app, token);
        }
        Ok(())
    });
}
pub fn exit_after_update(app: &tauri::AppHandle) {
    app.state::<Lifecycle>()
        .exiting
        .store(true, Ordering::SeqCst);
    app.exit(0);
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
    let action = match state.acknowledge(label, token, error.is_empty()) {
        FlushOutcome::Pending => return Ok(()),
        FlushOutcome::Failed => {
            resume_windows(app, token);
            if let Some(window) = app.get_webview_window(label) {
                let _ = window.show();
                let _ = window.set_focus();
            }
            return Err(error.into());
        }
        FlushOutcome::Complete(action) => action,
    };
    if action == Action::Update {
        return Ok(());
    }
    if action == Action::Quit {
        // Every window remains inert until process exit.
        app.exit(0);
    } else if let Some(window) = app.get_webview_window(label) {
        let result = if label == "main" {
            window.hide()
        } else {
            window.destroy()
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
                action: Action::Quit,
                completion: None,
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
            FlushOutcome::Complete(Action::Quit)
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
    #[test]
    fn update_waits_for_every_editor_and_keeps_admission_closed_until_install_finishes() {
        let state = request(9);
        let (sender, mut receiver) = tokio::sync::oneshot::channel();
        {
            let mut pending = state.pending.lock().unwrap();
            let request = pending.as_mut().unwrap();
            request.action = Action::Update;
            request.completion = Some(sender);
        }
        assert_eq!(state.acknowledge("main", 9, true), FlushOutcome::Pending);
        assert!(receiver.try_recv().is_err());
        assert_eq!(
            state.acknowledge("instant", 9, true),
            FlushOutcome::Complete(Action::Update)
        );
        assert_eq!(receiver.try_recv().unwrap(), Ok(9));
        assert!(!cancel_token_if(&state, 9, true));
        assert!(state
            .pending
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|r| r.action == Action::Update));
        assert!(!state.exiting.load(Ordering::SeqCst));
        assert_eq!(state.acknowledge("instant", 9, true), FlushOutcome::Pending);
        assert!(cancel_token(&state, 9));
        assert!(state.pending.lock().unwrap().is_none());
    }
    #[test]
    fn failed_or_timed_out_update_flush_never_grants_install_permission() {
        for success in [false, true] {
            let state = request(10);
            let (sender, mut receiver) = tokio::sync::oneshot::channel();
            {
                let mut pending = state.pending.lock().unwrap();
                let request = pending.as_mut().unwrap();
                request.action = Action::Update;
                request.completion = Some(sender);
            }
            state.acknowledge("main", 10, success);
            if success {
                assert!(cancel_token(&state, 10));
            }
            assert!(receiver.try_recv().unwrap().is_err());
            assert_eq!(
                state.acknowledge("instant", 10, true),
                FlushOutcome::Pending
            );
            assert!(!state.exiting.load(Ordering::SeqCst));
        }
    }
}
