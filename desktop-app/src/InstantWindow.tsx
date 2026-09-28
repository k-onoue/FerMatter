import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { save } from "@tauri-apps/plugin-dialog";
import { startDrag } from "@crabnebula/tauri-plugin-drag";
import {
  ClipboardCopy,
  Download,
  History,
  LoaderCircle,
  Pin,
  Plus,
  RefreshCw,
  Settings2,
  SunMoon,
  X,
} from "lucide-react";
import { asset, rpc } from "./api";
import { CodeEditor, configureTheme, setProjectCompletions } from "./editor";
import { defaults, palette } from "./settings";
import {
  builtinTemplates,
  initialTemplates,
  type TemplateConfiguration,
} from "./templates";
import { commandForEvent } from "./shortcuts";
import {
  initialInstant,
  instantError,
  instantRevision,
  readyInstant,
  type InstantDraft,
  type InstantHistory,
  type InstantSession,
} from "./instant";
import type { Card, Keybindings, Preferences, Theme } from "./types";
import { useDesktopBackground } from "./useDesktopBackground";
import "./instant.css";

const pendingKey = "mathlidelux.instant.pending.v1";
type Rendered = { card: Card; path: string; revision: string; image: string };
export default function InstantWindow() {
  const [draft, setDraft] = useState<InstantDraft | null>(null);
  const [history, setHistory] = useState<InstantHistory[]>([]);
  const [templates, setTemplates] =
    useState<TemplateConfiguration>(initialTemplates);
  const [rendered, setRendered] = useState<Rendered | null>(null);
  const [busy, setBusy] = useState(false),
    [outputBusy, setOutputBusy] = useState(false);
  const [compileError, setCompileError] = useState("");
  const [error, setError] = useState(""),
    [notice, setNotice] = useState("");
  const [details, setDetails] = useState(false),
    [historyOpen, setHistoryOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [theme, setTheme] = useState<Theme>(defaults);
  const [dark, setDark] = useState(true),
    [opacity, setOpacity] = useState(0.88),
    [pin, setPin] = useState(false);
  const [keys, setKeys] = useState<Keybindings>({
    schemaVersion: 1,
    kind: "mathlidelux.keybindings",
    bindings: {},
  });
  const [zoom, setZoom] = useState(1),
    [retry, setRetry] = useState(0);
  const draftRef = useRef(draft),
    queue = useRef(Promise.resolve());
  const saveError = useRef(""),
    sequence = useRef(0);
  const replacement = useRef(0);
  draftRef.current = draft;
  const report = (e: unknown) => {
    if (String(e) !== "Cancelled") setError(String(e));
  };
  const wallpaper = useDesktopBackground();
  const current = draft ? readyInstant(draft, rendered) : null;

  function persist(value: InstantDraft) {
    const text = instantRevision(value);
    try {
      localStorage.setItem(pendingKey, text);
    } catch (e) {
      report(e);
    }
    queue.current = queue.current
      .then(async () => {
        await rpc("instant_session", { action: "save", draft: value });
        saveError.current = "";
        if (localStorage.getItem(pendingKey) === text)
          localStorage.removeItem(pendingKey);
      })
      .catch((e) => {
        saveError.current = String(e);
        report(e);
      });
  }
  function change(value: InstantDraft) {
    draftRef.current = value;
    setDraft(value);
    persist(value);
  }
  const patch = (value: Partial<InstantDraft>) => {
    if (draftRef.current) change({ ...draftRef.current, ...value });
  };
  async function remember(value: InstantDraft) {
    const state = await rpc<InstantSession>("instant_session", {
      action: "remember",
      draft: value,
    });
    setHistory(state.history);
  }
  async function replace(value: InstantDraft) {
    const before = draftRef.current;
    const ticket = ++replacement.current;
    if (before?.source.trim()) await remember(before);
    if (replacement.current !== ticket || draftRef.current !== before) return;
    change(value);
    setCompileError("");
  }

  useEffect(() => {
    let gone = false;
    void Promise.all([
      rpc<{
        settings: Preferences;
        theme: Theme | null;
        keybindings: Keybindings;
        templates: TemplateConfiguration;
      }>("startup"),
      rpc<InstantSession>("instant_session", { action: "load" }),
    ])
      .then(async ([data, state]) => {
        if (gone) return;
        setTheme(data.theme || defaults);
        setDark(
          data.settings.appearance === "system"
            ? matchMedia("(prefers-color-scheme: dark)").matches
            : data.settings.appearance !== "light",
        );
        setOpacity(data.settings.opacity ?? 0.88);
        setKeys(data.keybindings);
        const configuration = Array.isArray(data.templates?.custom)
          ? data.templates
          : initialTemplates;
        setTemplates(configuration);
        setHistory(state.history);
        let next =
          state.draft ||
          initialInstant(
            [...builtinTemplates, ...configuration.custom].find(
              (t) => t.id === configuration.defaultID,
            ),
          );
        const pending = localStorage.getItem(pendingKey);
        if (pending) {
          next = JSON.parse(pending);
          // Validate recovery through the same backend before allowing further edits.
          await rpc("instant_session", { action: "save", draft: next });
          localStorage.removeItem(pendingKey);
        }
        if (!gone) {
          draftRef.current = next;
          setDraft(next);
        }
      })
      .catch(report);
    return () => {
      gone = true;
    };
  }, []);

  useEffect(() => {
    let gone = false;
    let cleanup: (() => void) | undefined;
    void getCurrentWindow()
      .onCloseRequested(async (event) => {
        await queue.current;
        if (saveError.current) {
          event.preventDefault();
          report(saveError.current);
          return;
        }
      })
      .then((fn) => {
        if (gone) fn();
        else cleanup = fn;
      })
      .catch(report);
    return () => {
      gone = true;
      cleanup?.();
    };
  }, []);

  useEffect(() => {
    const colors = palette(theme, dark);
    configureTheme(colors, dark);
    document.documentElement.dataset.appearance = dark ? "dark" : "light";
    for (const [key, value] of Object.entries(colors))
      document.documentElement.style.setProperty("--" + key, value);
    document.documentElement.style.setProperty("--opacity", String(opacity));
  }, [theme, dark, opacity]);
  useEffect(() => {
    void getCurrentWebview().setZoom(zoom).catch(report);
  }, [zoom]);
  useEffect(() => {
    if (!notice) return;
    const timer = setTimeout(() => setNotice(""), 2000);
    return () => clearTimeout(timer);
  }, [notice]);
  useEffect(() => {
    if (draft)
      setProjectCompletions("instant", [draft.template.source, draft.source]);
  }, [draft]);

  useEffect(() => {
    const ticket = ++sequence.current;
    setCompileError("");
    if (!draft?.source.trim()) {
      setRendered(null);
      setBusy(false);
      return;
    }
    const revision = instantRevision(draft);
    setBusy(true);
    const timer = setTimeout(() => {
      void rpc<{ card: Card; path: string }>("instant_render", {
        key: "render",
        draft,
      })
        .then(async (result) => {
          const image = await asset(result.card.svgPath!);
          if (sequence.current === ticket)
            setRendered({ ...result, revision, image });
        })
        .catch((e) => {
          if (sequence.current === ticket && String(e) !== "Cancelled")
            setCompileError(String(e));
        })
        .finally(() => {
          if (sequence.current === ticket) setBusy(false);
        });
    }, 450);
    return () => {
      sequence.current++;
      clearTimeout(timer);
      void rpc("cancel", { key: "render" }).catch(report);
    };
  }, [draft, retry]);

  async function copy() {
    if (!current || !draft) return;
    setOutputBusy(true);
    try {
      await rpc("copy_images", { paths: [current.path] });
      await remember(draft);
      setNotice("Copied");
    } catch (e) {
      report(e);
    } finally {
      setOutputBusy(false);
    }
  }
  async function exportImage() {
    if (!current || !draft) return;
    const snapshot = draft,
      result = current;
    setOutputBusy(true);
    try {
      const ext =
        snapshot.exportFormat === "jpeg" ? "jpg" : snapshot.exportFormat;
      const destination = await save({
        defaultPath: "equation." + ext,
        filters: [{ name: ext.toUpperCase(), extensions: [ext] }],
      });
      if (!destination) return;
      await rpc("instant_export", { path: result.path, destination });
      await remember(snapshot);
      setNotice("Exported");
    } catch (e) {
      report(e);
    } finally {
      setOutputBusy(false);
    }
  }
  function drag() {
    if (!current || !draft) return;
    void startDrag({
      item: [current.path],
      icon: current.card.pngPath!,
      mode: "copy",
    }).catch(report);
    void remember(draft).catch(report);
  }
  function togglePin() {
    void getCurrentWindow()
      .setAlwaysOnTop(!pin)
      .then(() => setPin(!pin))
      .catch(report);
  }
  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (event.isComposing) return;
      const command = commandForEvent(event, keys);
      let action: (() => void) | undefined;
      if (command === "copySVG") action = () => void copy();
      if (command === "exportSVG") action = () => void exportImage();
      if (command === "newEquation")
        action = () => {
          if (draft) void replace({ ...draft, source: "" }).catch(report);
        };
      if (command === "pin") action = togglePin;
      if (
        command === "instant" ||
        command === "rescan" ||
        (event.metaKey && event.key === "Enter")
      )
        action = () => setRetry((value) => value + 1);
      if (command === "saveEquation")
        action = () => {
          if (draft) {
            persist(draft);
            void queue.current.then(() => {
              if (!saveError.current) setNotice("Saved");
            });
          }
        };
      if (command === "zoomWindowIn")
        action = () => setZoom((value) => Math.min(2, value + 0.1));
      if (command === "zoomWindowOut")
        action = () => setZoom((value) => Math.max(0.5, value - 0.1));
      if (command === "resetWindowZoom") action = () => setZoom(1);
      if (event.metaKey && event.key === ",")
        action = () => setDetails((value) => !value);
      if (
        event.metaKey &&
        event.key.toLowerCase() === "c" &&
        !event.shiftKey &&
        !(event.target as HTMLElement).closest(
          'input,textarea,.monaco-editor,[contenteditable="true"]',
        )
      )
        action = () => void copy();
      if (action) {
        event.preventDefault();
        event.stopPropagation();
        action();
      }
    };
    window.addEventListener("keydown", handler, true);
    return () => window.removeEventListener("keydown", handler, true);
  }, [draft, current, keys, pin]);

  return (
    <div
      className="app instant-app"
      onDragOver={(event) => event.preventDefault()}
      onDrop={(event) => event.preventDefault()}
    >
      {wallpaper && (
        <div
          className="desktop-wallpaper"
          style={{ backgroundImage: `url(${wallpaper})` }}
        />
      )}
      <div className="titlebar" data-tauri-drag-region>
        <span data-tauri-drag-region>Mathlidelux · Instant</span>
      </div>
      <header className="instant-toolbar">
        <button
          aria-label="New equation"
          title="New equation (⌘N)"
          disabled={!draft}
          onClick={() =>
            draft && void replace({ ...draft, source: "" }).catch(report)
          }
        >
          <Plus size={18} />
        </button>
        <button
          aria-label="History"
          title="History"
          className={historyOpen ? "active" : ""}
          onClick={() => setHistoryOpen(!historyOpen)}
        >
          <History size={18} />
        </button>
        <span className="spacer" />
        <button
          aria-label="Always on top"
          title="Always on top"
          className={pin ? "active" : ""}
          onClick={togglePin}
        >
          <Pin size={17} />
        </button>
        <button
          aria-label="Appearance"
          title="Appearance"
          onClick={() => setDark(!dark)}
        >
          <SunMoon size={18} />
        </button>
        <button
          aria-label="Instant settings"
          title="Instant settings"
          className={details ? "active" : ""}
          onClick={() => setDetails(!details)}
        >
          <Settings2 size={18} />
        </button>
      </header>
      {!draft ? (
        <div className="empty">
          {error ? (
            <span>Unable to open Instant</span>
          ) : (
            <LoaderCircle className="spin" />
          )}
        </div>
      ) : (
        <>
          <div className="instant-options">
            <select
              aria-label="Template"
              value={draft.template.id}
              onChange={(event) => {
                const template = [
                  ...builtinTemplates,
                  ...templates.custom,
                ].find((t) => t.id === event.target.value);
                if (template)
                  void replace({
                    ...draft,
                    template,
                    kind: /tikz|cetz/.test(template.id) ? "figure" : "block",
                    source:
                      template.format === draft.template.format
                        ? draft.source
                        : "",
                  }).catch(report);
              }}
            >
              {[...builtinTemplates, ...templates.custom].map((t) => (
                <option key={t.id} value={t.id}>
                  {t.name}
                </option>
              ))}
              {![...builtinTemplates, ...templates.custom].some(
                (t) => t.id === draft.template.id,
              ) && (
                <option value={draft.template.id}>{draft.template.name}</option>
              )}
            </select>
            <select
              aria-label="Input mode"
              value={draft.kind}
              onChange={(event) =>
                patch({ kind: event.target.value as Card["kind"] })
              }
            >
              <option value="block">Display</option>
              <option value="inline">Inline</option>
              <option value="figure">Code</option>
            </select>
            <span className="spacer" />
            <button
              aria-label="Render"
              title="Render (⌘↵)"
              onClick={() => setRetry((value) => value + 1)}
            >
              <RefreshCw size={16} className={busy ? "spin" : ""} />
            </button>
          </div>
          <div className="instant-body">
            {historyOpen && (
              <aside className="instant-history">
                <input
                  aria-label="Search history"
                  placeholder="Search history"
                  value={query}
                  onChange={(event) => setQuery(event.target.value)}
                />
                <div>
                  {history
                    .filter((item) =>
                      (item.draft.source + item.draft.template.name)
                        .toLowerCase()
                        .includes(query.toLowerCase()),
                    )
                    .slice(0, 100)
                    .map((item) => (
                      <button
                        key={item.id}
                        onClick={() => void replace(item.draft).catch(report)}
                        title={item.draft.source}
                      >
                        <code>{item.draft.source}</code>
                        <small>
                          {item.draft.template.name} · {item.draft.fontSize} pt
                        </small>
                      </button>
                    ))}
                </div>
                {!history.length && <small>No history yet</small>}
              </aside>
            )}
            <main className="instant-workspace">
              {details && (
                <section className="instant-settings">
                  <div className="instant-options">
                    <strong>Template</strong>
                    <span className="spacer" />
                    {draft.template.format === "latex" && (
                      <select
                        aria-label="Compiler"
                        value={draft.template.compiler}
                        onChange={(event) =>
                          patch({
                            template: {
                              ...draft.template,
                              compiler: event.target.value,
                            },
                          })
                        }
                      >
                        {["pdflatex", "xelatex", "lualatex"].map((engine) => (
                          <option key={engine}>{engine}</option>
                        ))}
                      </select>
                    )}
                    <label>
                      Opacity{" "}
                      <input
                        aria-label="Window opacity"
                        type="range"
                        min="0.15"
                        max="1"
                        step="0.01"
                        value={opacity}
                        onChange={(event) => setOpacity(+event.target.value)}
                      />
                    </label>
                  </div>
                  <div className="instant-template-editor">
                    <CodeEditor
                      project="instant"
                      path={"template-" + draft.template.id}
                      format={draft.template.format}
                      value={draft.template.source}
                      onChange={(source) =>
                        patch({ template: { ...draft.template, source } })
                      }
                    />
                  </div>
                </section>
              )}
              <div className="instant-split">
                <section className="instant-editor">
                  <CodeEditor
                    project="instant"
                    path={
                      "equation." +
                      (draft.template.format === "latex" ? "tex" : "typ")
                    }
                    format={draft.template.format}
                    value={draft.source}
                    onChange={(source) => patch({ source })}
                    onSave={() => persist(draft)}
                  />
                </section>
                <section
                  className={`instant-preview ${draft.color.toLowerCase() === "#ffffff" ? "white-ink" : ""}`}
                  aria-label="Equation preview"
                  tabIndex={0}
                >
                  {rendered ? (
                    <img
                      src={rendered.image}
                      alt="Rendered equation"
                      draggable={!!current}
                      onDragStart={(event) => {
                        event.preventDefault();
                        drag();
                      }}
                    />
                  ) : (
                    <span className="instant-empty">Preview</span>
                  )}
                  <div className="instant-status" role="status">
                    {busy ? (
                      <>
                        <LoaderCircle size={14} className="spin" />
                        Rendering…
                      </>
                    ) : compileError ? (
                      "Needs correction"
                    ) : !current && draft.source.trim() ? (
                      "Preparing…"
                    ) : (
                      ""
                    )}
                  </div>
                </section>
              </div>
              {compileError && (
                <details className="instant-error">
                  <summary>{instantError(compileError)}</summary>
                  <pre>{compileError}</pre>
                </details>
              )}
            </main>
          </div>
          <footer className="instant-footer">
            <select
              aria-label="Export format"
              value={draft.exportFormat}
              onChange={(event) => patch({ exportFormat: event.target.value })}
            >
              {["svg", "pdf", "png", "jpeg"].map((format) => (
                <option key={format} value={format}>
                  {format.toUpperCase()}
                </option>
              ))}
            </select>
            <label className="instant-size">
              <input
                aria-label="Output font size"
                type="number"
                min="1"
                max="200"
                step="1"
                value={draft.fontSize}
                onChange={(event) => {
                  const value = +event.target.value;
                  if (value >= 1 && value <= 200) patch({ fontSize: value });
                }}
              />
              pt
            </label>
            <input
              aria-label="Equation color"
              title="Equation color"
              type="color"
              value={draft.color}
              onChange={(event) => patch({ color: event.target.value })}
            />
            <span className="spacer" />
            <button
              aria-label="Copy image"
              title="Copy image (⇧⌘C)"
              className="primary"
              disabled={!current || outputBusy}
              onClick={() => void copy()}
            >
              <ClipboardCopy size={17} />
              Copy
            </button>
            <button
              aria-label="Export image"
              title="Export image (⇧⌘S)"
              disabled={!current || outputBusy}
              onClick={() => void exportImage()}
            >
              <Download size={18} />
            </button>
          </footer>
        </>
      )}
      {error && (
        <div role="alert" className="toast error-toast">
          <div>{error}</div>
          <button aria-label="Dismiss error" onClick={() => setError("")}>
            <X size={17} />
          </button>
        </div>
      )}
      {notice && (
        <div className="toast" role="status">
          {notice}
        </div>
      )}
    </div>
  );
}
