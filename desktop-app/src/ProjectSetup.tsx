import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderOpen, LoaderCircle } from "lucide-react";
import { rpc } from "./api";
import type { Preferences, Project } from "./types";
import icon from "../src-tauri/icons/icon.png";

type SetupResult = { settings: Preferences; projects: Project[] };

export function ProjectSetup({
  initialDirectory,
  onComplete,
}: {
  initialDirectory: string;
  onComplete: (data: SetupResult) => void;
}) {
  const [directory, setDirectory] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  async function choose() {
    setError("");
    try {
      const path = await open({
        title: "Choose a project folder",
        directory: true,
        multiple: false,
        defaultPath: directory || initialDirectory,
      });
      if (typeof path === "string") setDirectory(path);
    } catch (e) {
      setError(String(e));
    }
  }
  async function finish() {
    if (busy || !directory) return;
    setBusy(true);
    setError("");
    try {
      onComplete(
        await rpc<SetupResult>("complete_project_setup", { directory }),
      );
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <main className="project-setup">
      <section aria-labelledby="setup-title">
        <img className="setup-icon" src={icon} alt="" />
        <h1 id="setup-title">Choose your project folder</h1>
        <p>New projects and imports will be saved here.</p>
        <button disabled={busy} onClick={() => void choose()}>
          <FolderOpen size={18} />
          Choose Folder…
        </button>
        {directory && (
          <div className="setup-path" title={directory}>
            {directory}
          </div>
        )}
        <p className="setup-demos">
          LaTeX, Typst, TikZ, and CeTZ demos will be added to a Demos folder.
        </p>
        {error && (
          <p role="alert" className="setup-error">
            {error}
          </p>
        )}
        <button
          className="primary"
          disabled={!directory || busy}
          onClick={() => void finish()}
        >
          {busy && <LoaderCircle size={18} className="spin" />}
          {busy ? "Setting up…" : "Continue"}
        </button>
      </section>
    </main>
  );
}
