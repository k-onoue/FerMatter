import { useEffect, useRef, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { rpc } from "./api";
import { CodeEditor } from "./editor";
import { Modal } from "./ui";
import type { SettingsSnapshot } from "./settingsPersistence";

export interface ExtractionConfiguration {
  schemaVersion: 1;
  kind: "mathlidelux.extraction";
  ignoreMathInTables: boolean;
  excludedLatexEnvironments: string[];
  excludedTypstFunctions: string[];
  [key: string]: unknown;
}

export function ExtractionSettings({
  onApplied,
}: {
  onApplied: (revision: string) => Promise<void>;
}) {
  const [snapshot, setSnapshot] =
    useState<SettingsSnapshot<ExtractionConfiguration> | null>(null);
  const [draft, setDraft] = useState<ExtractionConfiguration | null>(null);
  const [busy, setBusy] = useState(true);
  const [error, setError] = useState("");
  const [json, setJson] = useState<string | null>(null);
  const flight = useRef(false);
  const active = useRef(true);
  function accept(next: SettingsSnapshot<ExtractionConfiguration>) {
    if (!active.current) return;
    setSnapshot(next);
    setDraft(next.value);
    if (!next.revision)
      setError(
        "The settings file is invalid. Its original bytes were kept; repair it and Reload before saving.",
      );
  }
  useEffect(() => {
    active.current = true;
    let reading = true;
    void rpc<SettingsSnapshot<ExtractionConfiguration>>("extraction_settings")
      .then((next) => {
        if (reading) accept(next);
      })
      .catch((e) => {
        if (reading) setError(String(e));
      })
      .finally(() => {
        if (reading) setBusy(false);
      });
    return () => {
      reading = false;
      active.current = false;
    };
  }, []);
  async function request(action: () => Promise<void>) {
    if (flight.current) return;
    flight.current = true;
    setBusy(true);
    setError("");
    try {
      await action();
    } catch (e) {
      if (active.current) setError(String(e));
    } finally {
      flight.current = false;
      if (active.current) setBusy(false);
    }
  }
  async function apply() {
    if (!draft || !snapshot?.revision) return;
    await request(async () => {
      const revision = await rpc<string>("settings", {
        file: "extraction.json",
        value: draft,
        revision: snapshot.revision,
      });
      accept({ value: draft, revision });
      await onApplied(revision);
    });
  }
  const dirty =
    draft && JSON.stringify(draft) !== JSON.stringify(snapshot?.value);
  return (
    <>
      <h3>Card extraction</h3>
      <label className="setting-row">
        <span>Ignore math in tables</span>
        <input
          type="checkbox"
          checked={draft?.ignoreMathInTables === true}
          disabled={busy || !draft || !snapshot?.revision}
          onChange={(event) =>
            setDraft({ ...draft!, ignoreMathInTables: event.target.checked })
          }
        />
      </label>
      <p className="muted">
        Applies to Library cards in all projects. Source previews, original
        files and the full paper PDF are unchanged.
      </p>
      <p className="muted">
        Add custom LaTeX environment names and Typst function names in
        extraction.json. Apply or Reload refreshes open project libraries and
        preserves saved variants.
      </p>
      {error && (
        <p role="alert" className="setup-error">
          {error}
        </p>
      )}
      <div className="compiler-actions">
        <button
          disabled={busy || !draft}
          onClick={() => setJson(JSON.stringify(draft, null, 2))}
        >
          Edit JSON
        </button>
        <button
          disabled={busy}
          onClick={() =>
            void request(async () => {
              const path = await open({
                filters: [{ name: "JSON", extensions: ["json"] }],
                multiple: false,
              });
              if (typeof path === "string") {
                const value = await rpc<ExtractionConfiguration>("read_json", {
                  path,
                });
                setJson(JSON.stringify(value, null, 2));
              }
            })
          }
        >
          Import…
        </button>
        <button
          disabled={busy || !draft}
          onClick={() =>
            void request(async () => {
              const path = await save({
                defaultPath: "extraction.json",
                filters: [{ name: "JSON", extensions: ["json"] }],
              });
              if (path)
                await rpc("export_settings", {
                  file: "extraction.json",
                  path,
                  value: draft,
                });
            })
          }
        >
          Export…
        </button>
        <button
          disabled={busy}
          onClick={() =>
            void request(async () => {
              const next = await rpc<SettingsSnapshot<ExtractionConfiguration>>(
                "reload_settings",
                { file: "extraction.json" },
              );
              accept(next);
              await onApplied(next.revision);
            })
          }
        >
          Reload
        </button>
        <button
          className="primary"
          disabled={busy || !dirty || !snapshot?.revision}
          onClick={() => void apply()}
        >
          Apply and rescan
        </button>
      </div>
      <button
        disabled={busy}
        onClick={() =>
          void request(async () => {
            await rpc("reveal");
          })
        }
      >
        Reveal settings folder
      </button>
      {json !== null && (
        <Modal title="extraction.json" onClose={() => setJson(null)} wide>
          <div className="json-editor">
            <CodeEditor
              project="settings"
              path="extraction.json"
              value={json}
              format="json"
              onChange={setJson}
            />
          </div>
          <footer>
            <button
              className="primary"
              onClick={() => {
                try {
                  setDraft(JSON.parse(json));
                  setJson(null);
                } catch (e) {
                  setError(String(e));
                }
              }}
            >
              Use configuration
            </button>
          </footer>
        </Modal>
      )}
    </>
  );
}
