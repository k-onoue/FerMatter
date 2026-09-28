import { useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { FolderOpen, RotateCcw } from "lucide-react";
import { rpc } from "./api";
import { Modal } from "./ui";
import { CodeEditor } from "./editor";
import type { Keybindings, Preferences, Theme, Palette } from "./types";
import {
  commands,
  eventBinding,
  validateBindings,
  bindingKeys,
  effectiveBindings,
} from "./shortcuts";
export const defaults: Theme = {
  schemaVersion: 1,
  kind: "mathlidelux.theme",
  id: "org.mathlidelux.default",
  name: "Default",
  light: {
    accent: "#007AFF",
    windowBackground: "#EEF0F4",
    surfaceBackground: "#FFFFFF",
    text: "#20252D",
    command: "#8835B5",
    delimiter: "#078C94",
    number: "#BA5B00",
    comment: "#76818B",
  },
  dark: {
    accent: "#2997FF",
    windowBackground: "#1D232C",
    surfaceBackground: "#242B35",
    text: "#E7EDF5",
    command: "#C486F5",
    delimiter: "#65CCD0",
    number: "#F5B271",
    comment: "#8895A5",
  },
};
export function palette(theme: Theme | undefined, dark: boolean): Palette {
  const mode = dark ? "dark" : "light";
  const colors = { ...defaults[mode] };
  for (const [k, v] of Object.entries(theme?.[mode] || {}))
    if (/^#[0-9a-f]{6}$/i.test(v)) colors[k as keyof Palette] = v;
  return colors;
}
export function Settings({
  settings,
  themes,
  theme,
  keys,
  onSettings,
  onTheme,
  onKeys,
  onClose,
  onError,
}: {
  settings: Preferences;
  themes: Theme[];
  theme: Theme;
  keys: Keybindings;
  onSettings: (s: Preferences) => void;
  onTheme: (t: Theme) => void;
  onKeys: (k: Keybindings) => void;
  onClose: () => void;
  onError: (e: string) => void;
}) {
  const [tab, setTab] = useState("Appearance");
  const [record, setRecord] = useState("");
  const [json, setJson] = useState<{ file: string; text: string } | null>(null);
  const [tools, setTools] = useState<Record<string, string> | null>(null);
  const [search, setSearch] = useState("");
  const bindings = effectiveBindings(keys);
  const update = (patch: Partial<Preferences>) =>
    onSettings({ ...settings, ...patch });
  async function importFile(file: string) {
    try {
      const path = await open({
        filters: [{ name: "JSON", extensions: ["json"] }],
        multiple: false,
      });
      if (typeof path !== "string") return;
      const value = await rpc<Theme | Keybindings>("read_json", { path });
      if (file === "keybindings.json") {
        validateBindings(value as Keybindings);
        onKeys(value as Keybindings);
      } else {
        onTheme(value as Theme);
      }
    } catch (e) {
      onError(String(e));
    }
  }
  async function reload(file: string) {
    try {
      const value = await rpc<Theme | Keybindings>("reload_settings", { file });
      if (file === "theme.json") onTheme(value as Theme);
      else {
        validateBindings(value as Keybindings);
        onKeys(value as Keybindings);
      }
    } catch (error) {
      onError(String(error));
    }
  }
  async function exportSettings(file: string) {
    try {
      const path = await save({
        defaultPath: file,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (path)
        await rpc("export_settings", {
          file,
          path,
          value: file === "theme.json" ? theme : keys,
        });
    } catch (error) {
      onError(String(error));
    }
  }
  return (
    <Modal title="Settings" onClose={onClose} wide>
      <div className="settings-tabs">
        {["Appearance", "Editor", "Shortcuts", "Tools"].map((t) => (
          <button
            key={t}
            className={tab === t ? "active" : ""}
            onClick={() => setTab(t)}
          >
            {t}
          </button>
        ))}
      </div>
      <div className="settings-content">
        {tab === "Appearance" && (
          <>
            <label className="setting-row">
              <span>Appearance</span>
              <select
                value={settings.appearance}
                onChange={(e) =>
                  update({
                    appearance: e.target.value as Preferences["appearance"],
                  })
                }
              >
                {["system", "light", "dark"].map((a) => (
                  <option value={a} key={a}>
                    {a[0].toUpperCase() + a.slice(1)}
                  </option>
                ))}
              </select>
            </label>
            <label className="setting-row">
              <span>Color Theme</span>
              <select
                value={
                  [defaults, ...themes].some((t) => t.id === theme.id)
                    ? theme.id
                    : "custom"
                }
                onChange={(e) =>
                  onTheme(
                    [defaults, ...themes].find((t) => t.id === e.target.value)!,
                  )
                }
              >
                {[
                  defaults,
                  ...themes.sort((a, b) => a.name.localeCompare(b.name)),
                ].map((t) => (
                  <option key={t.id} value={t.id}>
                    {t.name}
                  </option>
                ))}
                <option value="custom" disabled>
                  Custom
                </option>
              </select>
            </label>
            <label className="setting-row">
              <span>Background opacity</span>
              <input
                type="range"
                min="0.45"
                max="1"
                step="0.01"
                value={settings.opacity}
                onChange={(e) => update({ opacity: +e.target.value })}
              />
              <span>{Math.round(settings.opacity * 100)}%</span>
            </label>
            <div className="setting-row">
              <span>Theme file</span>
              <button
                onClick={() =>
                  setJson({
                    file: "theme.json",
                    text: JSON.stringify(theme, null, 2),
                  })
                }
              >
                Edit JSON
              </button>
              <button onClick={() => void importFile("theme.json")}>
                Import…
              </button>
              <button onClick={() => void exportSettings("theme.json")}>
                Export…
              </button>
              <button onClick={() => void reload("theme.json")}>Reload</button>
            </div>
            <div
              className="theme-preview"
              style={{
                background: palette(theme, settings.appearance !== "light")
                  .surfaceBackground,
                color: palette(theme, settings.appearance !== "light").text,
              }}
            >
              <code>
                <span
                  style={{
                    color: palette(theme, settings.appearance !== "light")
                      .command,
                  }}
                >
                  \frac
                </span>
                {"{a}{b}"} + x²
              </code>
            </div>
          </>
        )}
        {tab === "Editor" && (
          <>
            <label className="setting-row">
              <span>Save files</span>
              <select
                value={settings.autoSave ? "auto" : "manual"}
                onChange={(e) =>
                  update({ autoSave: e.target.value === "auto" })
                }
              >
                <option value="auto">After changes</option>
                <option value="manual">Manually · ⌘ S</option>
              </select>
            </label>
            <p className="muted">
              Source edits update project files. Library edits create
              presentation variants.
            </p>
          </>
        )}
        {tab === "Shortcuts" && (
          <>
            <div className="shortcut-tools">
              <input
                placeholder="Search commands"
                aria-label="Search commands"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
              />
              <button
                onClick={() =>
                  setJson({
                    file: "keybindings.json",
                    text: JSON.stringify(keys, null, 2),
                  })
                }
              >
                Edit JSON
              </button>
              <button onClick={() => void importFile("keybindings.json")}>
                Import…
              </button>
              <button onClick={() => void exportSettings("keybindings.json")}>
                Export…
              </button>
              <button onClick={() => void reload("keybindings.json")}>
                Reload
              </button>
              <button
                title="Restore defaults"
                onClick={() =>
                  onKeys({
                    schemaVersion: 1,
                    kind: "mathlidelux.keybindings",
                    bindings: {},
                  })
                }
              >
                <RotateCcw size={16} />
              </button>
            </div>
            <div className="shortcuts">
              {commands
                .filter((c) =>
                  c.name.toLowerCase().includes(search.toLowerCase()),
                )
                .map((c) => {
                  const binding = bindings[c.id];
                  return (
                    <div className="shortcut-row" key={c.id}>
                      <span>{c.name}</span>
                      <button
                        className={`key-recorder ${record === c.id ? "recording" : ""}`}
                        onClick={() => setRecord(c.id)}
                        onKeyDown={(e) => {
                          e.preventDefault();
                          e.stopPropagation();
                          if (e.key === "Escape") {
                            setRecord("");
                            return;
                          }
                          if (
                            ["Meta", "Control", "Shift", "Alt"].includes(e.key)
                          )
                            return;
                          try {
                            const next = {
                              ...keys,
                              bindings: {
                                ...keys.bindings,
                                [c.id]: eventBinding(e),
                              },
                            };
                            validateBindings(next);
                            onKeys(next);
                            setRecord("");
                          } catch (error) {
                            onError(String(error));
                          }
                        }}
                      >
                        {record === c.id
                          ? "Press keys…"
                          : !binding.key
                            ? "Not set"
                            : bindingKeys(binding).map((key, i) => (
                                <kbd key={i}>{key}</kbd>
                              ))}
                      </button>
                      <button
                        title="Reset this shortcut"
                        onClick={() => {
                          const bindings = { ...keys.bindings };
                          delete bindings[c.id];
                          onKeys({ ...keys, bindings });
                        }}
                      >
                        <RotateCcw size={13} />
                      </button>
                    </div>
                  );
                })}
            </div>
          </>
        )}
        {tab === "Tools" && (
          <>
            <div className="setting-row">
              <span>Compiler tools</span>
              <button
                onClick={() =>
                  void rpc<Record<string, string>>("tools")
                    .then(setTools)
                    .catch((e) => onError(String(e)))
                }
              >
                Check
              </button>
            </div>
            {tools &&
              Object.entries(tools).map(([name, result]) => (
                <div className="tool-result" key={name}>
                  <strong>{name}</strong>
                  <pre>{result.trim() || "Available"}</pre>
                </div>
              ))}
            <div className="setting-row">
              <span>Settings folder</span>
              <button onClick={() => void rpc("reveal")}>
                <FolderOpen size={16} />
                Reveal in Finder
              </button>
            </div>
            <div className="muted">Mathlidelux</div>
          </>
        )}
      </div>
      {json && (
        <Modal title={json.file} onClose={() => setJson(null)} wide>
          <div className="json-editor">
            <CodeEditor
              project="settings"
              path={json.file}
              value={json.text}
              format="json"
              onChange={(text) => setJson({ ...json, text })}
            />
          </div>
          <footer>
            <button
              className="primary"
              onClick={() => {
                try {
                  const value = JSON.parse(json.text);
                  if (json.file === "keybindings.json") {
                    validateBindings(value);
                    onKeys(value);
                  } else onTheme(value);
                  setJson(null);
                } catch (e) {
                  onError(String(e));
                }
              }}
            >
              Save
            </button>
          </footer>
        </Modal>
      )}
    </Modal>
  );
}
