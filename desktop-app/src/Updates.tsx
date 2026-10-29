import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { listen } from "@tauri-apps/api/event";
import { Download } from "lucide-react";
import { rpc } from "./api";
import { Modal } from "./ui";

export type UpdateState = {
  revision: number;
  currentVersion: string;
  phase:
    | "idle"
    | "checking"
    | "current"
    | "available"
    | "downloading"
    | "ready"
    | "saving"
    | "installing";
  automatic: boolean;
  lastCheck: number;
  version: string | null;
  notes: string | null;
  downloaded: number;
  total: number;
  error: string | null;
  installAvailable: boolean;
};
export function newestUpdate(
  current: UpdateState | null,
  incoming: UpdateState,
) {
  return !current || incoming.revision >= current.revision ? incoming : current;
}
function useUpdates() {
  const [state, setState] = useState<UpdateState | null>(null);
  const [error, setError] = useState("");
  useEffect(() => {
    let active = true;
    const accept = (value: UpdateState) => {
      if (active) setState((current) => newestUpdate(current, value));
    };
    const stop = listen<UpdateState>("update-state", ({ payload }) =>
      accept(payload),
    );
    void stop
      .then(() => rpc<UpdateState>("update_state"))
      .then(accept)
      .catch((e) => {
        if (active) setError(String(e));
      });
    return () => {
      active = false;
      void stop.then((stop) => stop()).catch(() => {});
    };
  }, []);
  async function action(operation: string, payload = {}) {
    setError("");
    try {
      const value = await rpc<UpdateState>(operation, payload);
      setState((current) => newestUpdate(current, value));
    } catch (error) {
      setError(String(error));
    }
  }
  return { state, error, action };
}
export function UpdatesPanel() {
  const { state, error, action } = useUpdates();
  if (!state) return <p role="status">{error || "Loading update settings…"}</p>;
  const busy = ["checking", "downloading", "saving", "installing"].includes(
    state.phase,
  );
  const messages = {
    idle: "Check for a newer stable version.",
    checking: "Checking for updates…",
    current: "No newer stable version is available for this platform.",
    available: `FerMatter ${state.version} is available.`,
    downloading: "Downloading update…",
    ready: `FerMatter ${state.version} is ready to install.`,
    saving: "Saving all open editors…",
    installing: "Installing update. FerMatter will restart…",
  };
  return (
    <div className="updates-panel">
      <p>
        Installed version: <strong>{state.currentVersion}</strong>
      </p>
      <p role="status">{messages[state.phase]}</p>
      {(error || state.error) && (
        <p role="alert" className="update-error">
          {error || state.error}
        </p>
      )}
      {state.notes && <p className="update-notes">{state.notes}</p>}
      {state.phase === "downloading" && (
        <progress
          aria-label="Update download"
          max={state.total || 1}
          value={state.downloaded}
        />
      )}
      <div className="update-actions">
        <button
          disabled={busy || state.phase === "ready"}
          onClick={() => void action("update_check")}
        >
          Check for updates
        </button>
        {state.phase === "available" && (
          <button
            className="primary"
            disabled={!state.installAvailable}
            onClick={() => void action("update_download")}
          >
            Download update
          </button>
        )}
        {state.phase === "ready" && (
          <button
            className="primary"
            disabled={!state.installAvailable}
            onClick={() => void action("update_install")}
          >
            Install and restart
          </button>
        )}
        {["checking", "downloading"].includes(state.phase) && (
          <button onClick={() => void action("update_cancel")}>Cancel</button>
        )}
      </div>
      {!state.installAvailable && (
        <p>
          In-app installation requires an installed release build. On macOS,
          open the copy in Applications.
        </p>
      )}
      {state.phase === "ready" && (
        <p>
          All open editors will be saved before installation. The operating
          system may ask you to approve the installation.
        </p>
      )}
      <label className="setting-row">
        <span>Automatically check for updates</span>
        <input
          type="checkbox"
          checked={state.automatic}
          disabled={busy}
          onChange={(event) =>
            void action("update_automatic", { enabled: event.target.checked })
          }
        />
      </label>
      <p className="update-detail">
        Checks contact GitHub Pages, at most once a day when enabled. Documents,
        equations, settings and usage data are never sent. Downloads and
        installation always require your action.
      </p>
      <button onClick={() => void action("update_history")}>
        Resolved issues ↗
      </button>
    </div>
  );
}
export function UpdatesButton() {
  const { state } = useUpdates();
  const [open, setOpen] = useState(false);
  const available = !!state?.version;
  return (
    <>
      <button
        className={available ? "update-available" : ""}
        title={available ? `Update available: ${state.version}` : "Updates"}
        aria-label={
          available ? `Update available: ${state.version}` : "Updates"
        }
        onClick={() => setOpen(true)}
      >
        <Download size={17} />
        {available && <span className="update-dot" aria-hidden="true" />}
      </button>
      <span className="visually-hidden" role="status">
        {available
          ? `FerMatter ${state.version} is available. Open Updates to install it.`
          : ""}
      </span>
      {open &&
        createPortal(
          <Modal title="Updates" onClose={() => setOpen(false)}>
            <UpdatesPanel />
          </Modal>,
          document.body,
        )}
    </>
  );
}
