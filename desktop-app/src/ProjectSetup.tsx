import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderOpen, LoaderCircle, Zap } from "lucide-react";
import { rpc } from "./api";
import type { Preferences, Project } from "./types";
import type { SettingsSnapshot } from "./settingsPersistence";
import { BrandMark } from "./BrandMark";
import { CompilerToolFields, useCompilerTools } from "./CompilerTools";
import { isWindows } from "./platform";

type SetupResult = {
  settings: SettingsSnapshot<Preferences>;
  projects: Project[];
};

export function ProjectSetup({
  initialDirectory,
  onComplete,
}: {
  initialDirectory: string;
  onComplete: (data: SetupResult) => void;
}) {
  const [chosenDirectory, setDirectory] = useState<string>();
  const directory = chosenDirectory ?? (isWindows() ? initialDirectory : "");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [step, setStep] = useState<"folder" | "tools">("folder");
  const tools = useCompilerTools();
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
      if (!(await tools.save())) return;
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
        <BrandMark className="setup-icon" />
        <h1 id="setup-title">
          {step === "folder" ? "Choose your project folder" : "Compiler tools"}
        </h1>
        {step === "folder" ? (
          <>
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
              LaTeX, Typst, TikZ, and CeTZ demos will be added to a Demos
              folder.
            </p>
            {error && (
              <p role="alert" className="setup-error">
                {error}
              </p>
            )}
            <button
              className="primary"
              disabled={!directory}
              onClick={() => setStep("tools")}
            >
              Next
            </button>
            <button
              onClick={() =>
                void rpc("instant_open").catch((e) => setError(String(e)))
              }
            >
              <Zap size={18} /> Open Instant
            </button>
          </>
        ) : (
          <>
            <CompilerToolFields state={tools} disabled={busy} />
            {error && (
              <p role="alert" className="setup-error">
                {error}
              </p>
            )}
            <div className="compiler-actions">
              <button
                disabled={busy || tools.busy}
                onClick={() => setStep("folder")}
              >
                Back
              </button>
              <button
                disabled={busy || tools.busy || !tools.draft}
                onClick={() => void tools.check()}
              >
                Check paths
              </button>
              <button
                className="primary"
                disabled={!directory || busy || tools.busy || !tools.draft}
                onClick={() => void finish()}
              >
                {busy && <LoaderCircle size={18} className="spin" />}
                {busy ? "Setting up…" : "Finish Setup"}
              </button>
            </div>
          </>
        )}
      </section>
    </main>
  );
}
