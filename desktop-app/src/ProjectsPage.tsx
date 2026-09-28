import { useEffect, useRef, useState } from "react";
import {
  ArrowUpRight,
  ChevronDown,
  Copy,
  FolderOpen,
  Grid2X2,
  List,
  LoaderCircle,
  Plus,
  Redo2,
  Search,
  Settings2,
  SunMoon,
  Trash2,
  Undo2,
  Zap,
} from "lucide-react";
import { rpc, projectName } from "./api";
import { ContextMenu, ImagePreview, Modal } from "./ui";
import { stepZoom } from "./scrollZoom";
import { useControlScroll } from "./useControlScroll";
import { projectSelection } from "./projectSelection";
import type { Preferences, Project } from "./types";
import icon from "../src-tauri/icons/icon.png";
type History = {
  undo: string | null;
  redo: string | null;
  undoTrash: string[];
  redoTrash: string[];
};
type Confirmation = { action: string; roots: string[]; ids?: string[] };
export function ProjectsPage({
  projects,
  onProjects,
  settings,
  onSettings,
  onOpen,
  onOpenDialog,
  onNew,
  onSettingsDialog,
  onInstant,
  onError,
  onNotice,
  defaultDirectory,
  blocked,
}: {
  projects: Project[];
  onProjects: (p: Project[]) => void;
  settings: Preferences;
  onSettings: (p: Preferences) => void;
  onOpen: (p: Project) => void;
  onOpenDialog: (type?: string) => void;
  onNew: () => void;
  onSettingsDialog: () => void;
  onInstant: () => void;
  onError: (e: string) => void;
  onNotice: (n: string) => void;
  defaultDirectory: string;
  blocked: boolean;
}) {
  const [search, setSearch] = useState("");
  const [zoom, setZoom] = useState(1);
  const scroll = useControlScroll((steps) =>
    setZoom((z) => stepZoom(z, steps)),
  );
  const [selected, setSelected] = useState(new Set<string>());
  const anchor = useRef<string>(undefined);
  const grid = useRef<HTMLDivElement>(null);
  const [history, setHistory] = useState<History>({
    undo: null,
    redo: null,
    undoTrash: [],
    redoTrash: [],
  });
  const [busy, setBusy] = useState(false),
    working = useRef(false);
  const [dropping, setDropping] = useState(false);
  const [confirmation, setConfirmation] = useState<Confirmation | null>(null);
  const [menu, setMenu] = useState<{
    x: number;
    y: number;
    open?: boolean;
    ids?: string[];
  } | null>(null);
  const [marquee, setMarquee] = useState<{
    x: number;
    y: number;
    width: number;
    height: number;
  } | null>(null);
  const visible = projects.filter((p) =>
    [projectName(p.root), p.root].some((t) =>
      t.toLowerCase().includes(search.toLowerCase()),
    ),
  );
  const chosen = visible.filter((p) => selected.has(p.id));
  useEffect(() => {
    void rpc<History>("project_history")
      .then(setHistory)
      .catch((e) => onError(String(e)));
  }, []);
  useEffect(() => {
    setSelected(
      (old) =>
        new Set([...old].filter((id) => visible.some((p) => p.id === id))),
    );
  }, [projects, search]);
  async function run(action: string, payload: Record<string, unknown> = {}) {
    if (working.current) return;
    working.current = true;
    setBusy(true);
    setMenu(null);
    setConfirmation(null);
    try {
      const result = await rpc<{
        projects: Project[];
        history: History;
        created: string[];
        errors: string[];
      }>("project_action", { action, ...payload });
      onProjects(result.projects);
      setHistory(result.history);
      if (result.created.length) setSearch("");
      setSelected(
        new Set(
          result.projects
            .filter((p) => result.created.includes(p.root))
            .map((p) => p.id),
        ),
      );
      if (result.errors.length) onError(result.errors.join("\n"));
      else
        onNotice(
          action === "import"
            ? "Projects imported"
            : action === "duplicate"
              ? "Projects duplicated"
              : action === "delete"
                ? "Moved to Trash"
                : action === "undo"
                  ? "Undone"
                  : "Redone",
        );
    } catch (e) {
      onError(String(e));
    } finally {
      working.current = false;
      setBusy(false);
    }
  }
  function request(action: string, ids = chosen.map((p) => p.id)) {
    if (busy || blocked || confirmation) return;
    const roots =
      action === "delete"
        ? projects.filter((p) => ids.includes(p.id)).map((p) => p.root)
        : action === "undo"
          ? history.undoTrash
          : action === "redo"
            ? history.redoTrash
            : [];
    if (roots.length) setConfirmation({ action, roots, ids });
    else void run(action, { ids });
  }
  function choose(
    id: string,
    e: { shiftKey: boolean; metaKey: boolean; ctrlKey: boolean },
  ) {
    setSelected(
      projectSelection(
        visible.map((p) => p.id),
        selected,
        id,
        anchor.current,
        e,
      ),
    );
    if (!e.shiftKey || !anchor.current) anchor.current = id;
  }
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (
        blocked ||
        busy ||
        confirmation ||
        e.isComposing ||
        (e.target as HTMLElement).closest(
          'input,textarea,[contenteditable="true"],[role="dialog"]',
        )
      )
        return;
      const command = e.metaKey || e.ctrlKey;
      if (command && !e.altKey && e.key.toLowerCase() === "a") {
        e.preventDefault();
        setSelected(new Set(visible.map((p) => p.id)));
      } else if (command && !e.altKey && e.key.toLowerCase() === "z") {
        e.preventDefault();
        if (e.shiftKey ? history.redo : history.undo)
          request(e.shiftKey ? "redo" : "undo");
      } else if (command && !e.altKey && e.key.toLowerCase() === "d") {
        e.preventDefault();
        if (chosen.length) request("duplicate");
      } else if (command && e.key === "Backspace") {
        e.preventDefault();
        if (chosen.length) request("delete");
      } else if (
        e.key === "Enter" &&
        chosen.length === 1 &&
        !(e.target as HTMLElement).closest("button")
      ) {
        e.preventDefault();
        onOpen(chosen[0]);
      } else if (e.key === "Escape") {
        setSelected(new Set());
        setMenu(null);
      } else if (
        !command &&
        ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(e.key)
      ) {
        e.preventDefault();
        const elements = [
          ...(grid.current?.querySelectorAll<HTMLElement>(
            "[data-project-id]",
          ) || []),
        ];
        const columns =
          settings.projectView === "list"
            ? 1
            : Math.max(
                1,
                elements.filter((el) => el.offsetTop === elements[0]?.offsetTop)
                  .length,
              );
        const focused = (document.activeElement as HTMLElement)?.dataset
          .projectId;
        const current = visible.findIndex(
          (p) => p.id === (focused || chosen.at(-1)?.id),
        );
        const delta =
          e.key === "ArrowLeft"
            ? -1
            : e.key === "ArrowRight"
              ? 1
              : e.key === "ArrowUp"
                ? -columns
                : columns;
        const next =
          visible[
            Math.max(
              0,
              Math.min(visible.length - 1, current < 0 ? 0 : current + delta),
            )
          ];
        if (next) {
          choose(next.id, e);
          elements.find((el) => el.dataset.projectId === next.id)?.focus();
        }
      }
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  });
  return (
    <main
      className={`projects-page ${dropping ? "project-drop-active" : ""}`}
      ref={scroll}
      onDragOver={(e) => {
        e.preventDefault();
        if (
          !blocked &&
          !busy &&
          !confirmation &&
          e.dataTransfer.types.includes("Files")
        ) {
          e.dataTransfer.dropEffect = "copy";
          setDropping(true);
        }
      }}
      onDragLeave={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget as Node | null))
          setDropping(false);
      }}
      onDrop={(e) => {
        e.preventDefault();
        setDropping(false);
        if (blocked || busy || confirmation) return;
        const names = Array.from(e.dataTransfer.files).map((f) => f.name);
        if (!names.length) return;
        void rpc<string[]>("dropped_project_paths", { names })
          .then((paths) => run("import", { paths }))
          .catch((e) => onError(String(e)));
      }}
    >
      <div className="brand">
        <img src={icon} />
        <strong>Mathlidelux</strong>
        <span className="spacer" />
        <button
          title="Appearance"
          onClick={() =>
            onSettings({
              ...settings,
              appearance:
                document.documentElement.dataset.appearance === "dark"
                  ? "light"
                  : "dark",
            })
          }
        >
          <SunMoon size={19} />
        </button>
        <button title="Settings" onClick={onSettingsDialog}>
          <Settings2 size={19} />
        </button>
      </div>
      <header className="projects-heading">
        <h1>Projects</h1>
        <button className="soft" title="New Instant (⇧⌘K)" onClick={onInstant}>
          <Zap size={17} />
          Instant
        </button>
        <span className="spacer" />
        <button
          className="zoom-level"
          title="Reset project zoom (Ctrl + scroll to zoom)"
          aria-label={`Project zoom ${Math.round(zoom * 100)}%. Reset to 100%`}
          onClick={() => setZoom(1)}
        >
          {Math.round(zoom * 100)}%
        </button>
        <div
          className="segmented project-view-switch"
          role="group"
          aria-label="Project view"
        >
          {(["grid", "list"] as const).map((view) => (
            <button
              key={view}
              aria-label={`${view === "grid" ? "Grid" : "List"} view`}
              aria-pressed={settings.projectView === view}
              className={settings.projectView === view ? "active" : ""}
              onClick={() => onSettings({ ...settings, projectView: view })}
            >
              {view === "grid" ? <Grid2X2 size={17} /> : <List size={17} />}
            </button>
          ))}
        </div>
        <div className="search">
          <Search size={16} />
          <input
            placeholder="Search projects"
            aria-label="Search projects"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>
        <button className="soft" onClick={() => onOpenDialog()}>
          <FolderOpen size={17} />
          Open Project
        </button>
        <button
          title="Open file or ZIP"
          onClick={(e) => setMenu({ x: e.clientX, y: e.clientY, open: true })}
        >
          <ChevronDown size={15} />
        </button>
        <button className="primary" onClick={onNew}>
          <Plus size={17} />
          New Project
        </button>
      </header>
      <div className="project-actions">
        <span>
          {chosen.length
            ? `${chosen.length} selected`
            : `${visible.length} projects`}
        </span>
        <span className="spacer" />
        <button
          disabled={busy || !history.undo}
          title={history.undo ? `Undo ${history.undo} (⌘Z)` : "Undo (⌘Z)"}
          onClick={() => request("undo")}
        >
          <Undo2 size={17} />
        </button>
        <button
          disabled={busy || !history.redo}
          title={history.redo ? `Redo ${history.redo} (⇧⌘Z)` : "Redo (⇧⌘Z)"}
          onClick={() => request("redo")}
        >
          <Redo2 size={17} />
        </button>
        <button
          disabled={busy || !chosen.length}
          title="Duplicate projects (⌘D)"
          onClick={() => request("duplicate")}
        >
          <Copy size={17} />
        </button>
        <button
          disabled={busy || !chosen.length}
          title="Delete projects (⌘⌫)"
          onClick={() => request("delete")}
        >
          <Trash2 size={17} />
        </button>
      </div>
      <div
        className={`project-grid ${settings.projectView === "list" ? "project-list" : ""}`}
        style={{ zoom }}
        role="grid"
        aria-label="Projects"
        aria-multiselectable
        tabIndex={0}
        ref={grid}
        onPointerDown={(e) => {
          if (
            e.button !== 0 ||
            (e.target as HTMLElement).closest(".project-card,button")
          )
            return;
          e.currentTarget.focus();
          const start = { x: e.clientX, y: e.clientY };
          const previous =
            e.metaKey || e.ctrlKey || e.shiftKey
              ? new Set(selected)
              : new Set<string>();
          setSelected(previous);
          const el = e.currentTarget;
          el.setPointerCapture(e.pointerId);
          const move = (ev: PointerEvent) => {
            const box = {
              x: Math.min(start.x, ev.clientX),
              y: Math.min(start.y, ev.clientY),
              width: Math.abs(start.x - ev.clientX),
              height: Math.abs(start.y - ev.clientY),
            };
            setMarquee(box);
            const next = new Set(previous);
            el.querySelectorAll<HTMLElement>("[data-project-id]").forEach(
              (card) => {
                const r = card.getBoundingClientRect();
                if (
                  r.left < box.x + box.width &&
                  r.right > box.x &&
                  r.top < box.y + box.height &&
                  r.bottom > box.y
                )
                  next.add(card.dataset.projectId!);
              },
            );
            setSelected(next);
          };
          const end = () => {
            el.removeEventListener("pointermove", move);
            el.removeEventListener("pointerup", end);
            el.removeEventListener("pointercancel", end);
            setMarquee(null);
          };
          el.addEventListener("pointermove", move);
          el.addEventListener("pointerup", end);
          el.addEventListener("pointercancel", end);
        }}
      >
        {visible.map((p) => (
          <article
            key={p.id}
            className={`project-card ${selected.has(p.id) ? "selected" : ""}`}
            role="row"
            aria-label={projectName(p.root)}
            aria-selected={selected.has(p.id)}
            tabIndex={0}
            data-project-id={p.id}
            onClick={(e) => choose(p.id, e)}
            onDoubleClick={() => !busy && onOpen(p)}
            onContextMenu={(e) => {
              e.preventDefault();
              const ids = selected.has(p.id) ? chosen.map((p) => p.id) : [p.id];
              setSelected(new Set(ids));
              setMenu({ x: e.clientX, y: e.clientY, ids });
            }}
          >
            <div role="gridcell" className="project-card-content">
              <ImagePreview card={p.cards?.find((c) => c.status === "ready")} />
              <div className="project-name">
                <h2>{projectName(p.root)}</h2>
                <div className="project-card-actions">
                  <button
                    title="Open with Instant Mode"
                    aria-label={`Open ${projectName(p.root)} with Instant Mode`}
                    disabled={busy}
                    onDoubleClick={(e) => e.stopPropagation()}
                    onClick={(e) => {
                      e.stopPropagation();
                      void rpc("instant_open", { project: p.id }).catch((e) =>
                        onError(String(e)),
                      );
                    }}
                  >
                    <Zap size={18} />
                  </button>
                  <button
                    aria-label={`Open ${projectName(p.root)}`}
                    onDoubleClick={(e) => e.stopPropagation()}
                    onClick={(e) => {
                      e.stopPropagation();
                      if (!busy) onOpen(p);
                    }}
                  >
                    <ArrowUpRight size={18} />
                  </button>
                </div>
              </div>
              <p className="project-location" title={p.root}>
                {p.root}
              </p>
              <p className="project-details">
                <span className="project-format">
                  {p.format === "typst" ? "Typst" : "LaTeX"}
                </span>
                <span className="project-detail-separator"> · </span>
                <span className="project-count">
                  {p.cards?.length || 0} items
                </span>
              </p>
            </div>
          </article>
        ))}
        <button className="new-project-card" onClick={onNew}>
          <Plus size={30} />
          New Project
        </button>
      </div>
      {(dropping || busy) && (
        <div className="project-drop-overlay">
          {busy ? <LoaderCircle className="spin" /> : <FolderOpen />}
          <strong>{busy ? "Working…" : "Import Projects"}</strong>
          {!busy && <span>{defaultDirectory}</span>}
        </div>
      )}
      {marquee && (
        <div
          className="marquee"
          style={{
            left: marquee.x,
            top: marquee.y,
            width: marquee.width,
            height: marquee.height,
          }}
        />
      )}
      {confirmation && (
        <Modal
          title={`Delete ${confirmation.roots.length === 1 ? "project" : "projects"}?`}
          onClose={() => setConfirmation(null)}
        >
          <div className="delete-project-details">
            <p>Move these folders to Trash? You can undo this action.</p>
            <ul>
              {confirmation.roots.map((root) => (
                <li key={root}>
                  <strong>{projectName(root)}</strong>
                  <span>{root}</span>
                </li>
              ))}
            </ul>
          </div>
          <footer>
            <button autoFocus onClick={() => setConfirmation(null)}>
              Cancel
            </button>
            <button
              className="danger"
              onClick={() =>
                void run(confirmation.action, { ids: confirmation.ids })
              }
            >
              Move to Trash
            </button>
          </footer>
        </Modal>
      )}
      {menu && (
        <ContextMenu
          x={menu.x}
          y={menu.y}
          onClose={() => setMenu(null)}
          items={
            menu.open
              ? [
                  { label: "Open Folder…", action: () => onOpenDialog() },
                  {
                    label: "Open Main File…",
                    action: () => onOpenDialog("file"),
                  },
                  { label: "Import ZIP…", action: () => onOpenDialog("zip") },
                ]
              : [
                  {
                    label: "Open Project",
                    disabled: menu.ids?.length !== 1,
                    action: () =>
                      onOpen(projects.find((p) => p.id === menu.ids?.[0])!),
                  },
                  {
                    label: "Open with Instant Mode",
                    disabled: menu.ids?.length !== 1,
                    action: () =>
                      void rpc("instant_open", {
                        project: menu.ids?.[0],
                      }).catch((e) => onError(String(e))),
                  },
                  {
                    label: "Reveal in Finder",
                    action: () => {
                      for (const id of menu.ids || [])
                        void rpc("reveal", { project: id }).catch((e) =>
                          onError(String(e)),
                        );
                    },
                  },
                  {
                    label: "Duplicate",
                    disabled: busy,
                    action: () => request("duplicate", menu.ids),
                  },
                  {
                    label: "Delete…",
                    disabled: busy,
                    action: () => request("delete", menu.ids),
                  },
                  {
                    label: history.undo ? `Undo ${history.undo}` : "Undo",
                    disabled: busy || !history.undo,
                    action: () => request("undo"),
                  },
                  {
                    label: history.redo ? `Redo ${history.redo}` : "Redo",
                    disabled: busy || !history.redo,
                    action: () => request("redo"),
                  },
                ]
          }
        />
      )}
    </main>
  );
}
