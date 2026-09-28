import { useState, useEffect, useRef, useCallback } from "react";
import { createPortal } from "react-dom";
import {
  File,
  Folder,
  ChevronRight,
  ChevronDown,
  X,
  Columns2,
  Rows2,
  Plus,
  Search,
  ClipboardCopy,
  RefreshCw,
  TriangleAlert,
} from "lucide-react";
import { startDrag } from "@crabnebula/tauri-plugin-drag";
import { CodeEditor, setProjectCompletions } from "./editor";
import { rpc, uid } from "./api";
import { ImagePreview, ContextMenu, type Ask } from "./ui";
import type { Project, Buffer, TreeEntry, Card } from "./types";
import {
  leaf,
  leaves,
  update,
  removePane,
  type Leaf,
  type Pane,
} from "./panes";
function PaneView({
  pane,
  renderLeaf,
  onRatio,
}: {
  pane: Pane;
  renderLeaf: (p: Leaf) => React.ReactNode;
  onRatio: (id: string, ratio: number) => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  if (!("children" in pane)) return renderLeaf(pane);
  return (
    <div ref={ref} className={`editor-split ${pane.direction}`}>
      <div style={{ flex: `${pane.ratio} 1 0` }}>
        <PaneView
          pane={pane.children[0]}
          renderLeaf={renderLeaf}
          onRatio={onRatio}
        />
      </div>
      <div
        className="split-resizer"
        onPointerDown={(e) => {
          e.preventDefault();
          const el = e.currentTarget;
          el.setPointerCapture(e.pointerId);
          const bounds = ref.current!.getBoundingClientRect();
          const move = (event: PointerEvent) =>
            onRatio(
              pane.id,
              Math.max(
                0.15,
                Math.min(
                  0.85,
                  pane.direction === "row"
                    ? (event.clientX - bounds.left) / bounds.width
                    : (event.clientY - bounds.top) / bounds.height,
                ),
              ),
            );
          el.addEventListener("pointermove", move);
          el.addEventListener(
            "pointerup",
            () => el.removeEventListener("pointermove", move),
            { once: true },
          );
        }}
      />
      <div style={{ flex: `${1 - pane.ratio} 1 0` }}>
        <PaneView
          pane={pane.children[1]}
          renderLeaf={renderLeaf}
          onRatio={onRatio}
        />
      </div>
    </div>
  );
}
export function SourceWorkspace({
  project,
  autoSave,
  onDrafts,
  onError,
  ask,
  navigate,
  onSaved,
  onCursor,
}: {
  project: Project;
  autoSave: boolean;
  onDrafts: (sources: Record<string, string>) => void;
  onError: (message: string) => void;
  ask: Ask;
  navigate?: {
    path: string;
    line: number;
    nonce: number;
    startUTF16?: number;
    endUTF16?: number;
  };
  onSaved: () => void;
  onCursor: (file: string, offset: number, line: number) => void;
}) {
  const [tree, setTree] = useState<TreeEntry[]>([]);
  const [buffers, setBuffers] = useState<Record<string, Buffer>>({});
  const [draftsLoaded, setDraftsLoaded] = useState(false);
  const draftsHydrated = useRef(false);
  const first = useRef(leaf());
  const [layout, setLayout] = useState<Pane>(first.current);
  const [activePane, setActivePane] = useState(first.current.id);
  const [expanded, setExpanded] = useState(new Set<string>());
  const [treeMode, setTreeMode] = useState<"files" | "outline">("files");
  const [compactNavigation, setCompactNavigation] = useState(false);
  const navigationElement = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const element = navigationElement.current;
    if (!element) return;
    const observer = new ResizeObserver(([entry]) => {
      if (entry.contentRect.height > 0)
        setCompactNavigation(entry.contentRect.height < 360);
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  const [query, setQuery] = useState("");
  const [menu, setMenu] = useState<{
    x: number;
    y: number;
    path: string;
    directory: boolean;
  } | null>(null);
  const [clipboard, setClipboard] = useState<{
    path: string;
    cut: boolean;
  } | null>(null);
  const [preview, setPreview] = useState<Card | null>(null);
  const [previewBusy, setPreviewBusy] = useState(false);
  const [caret, setCaret] = useState<{
    file: string;
    offset: number;
    line: number;
  } | null>(null);
  const [reveal, setReveal] = useState<{
    file: string;
    line: number;
    nonce: number;
    startUTF16?: number;
    endUTF16?: number;
  } | null>(null);
  const [drop, setDrop] = useState<{ pane: string; edge: string } | null>(null);
  const [previewSize, setPreviewSize] = useState(0);
  const refs = useRef({ buffers, layout, activePane });
  refs.current = { buffers, layout, activePane };
  const completionSources = useRef<string[]>([]);
  const savedCallback = useRef(onSaved);
  savedCallback.current = onSaved;
  const pending = useRef(new Map<string, Promise<void>>());
  const mounted = useRef(true);
  const fileOpens = useRef(new Map<string, number>());
  const ticket = useRef(0);
  const sources = Object.fromEntries(
    Object.values(buffers)
      .filter((b) => b.dirty)
      .map((b) => [b.path, b.text]),
  );
  const fail = (e: unknown) => onError(String(e));
  const refresh = useCallback(
    () =>
      rpc<TreeEntry[]>("tree", { project: project.id })
        .then(setTree)
        .catch(onError),
    [project.id],
  );
  const save = useCallback(
    async (path: string) => {
      if (pending.current.has(path)) return pending.current.get(path);
      const buffer = refs.current.buffers[path];
      if (!buffer?.dirty) return;
      const promise = rpc<Buffer>("save_file", { project: project.id, buffer })
        .then((saved) => {
          if (!mounted.current) return;
          setBuffers((current) => {
            const now = current[path];
            if (!now) return current;
            return {
              ...current,
              [path]: {
                ...saved,
                text: now.text,
                dirty: now.text !== buffer.text,
                error: undefined,
              },
            };
          });
          savedCallback.current();
        })
        .catch((e) => {
          if (mounted.current)
            setBuffers((current) => ({
              ...current,
              [path]: { ...current[path], error: String(e) },
            }));
          onError(String(e));
        })
        .finally(() => pending.current.delete(path));
      pending.current.set(path, promise);
      return promise;
    },
    [project.id],
  );
  const open = useCallback(
    async (path: string, pane?: string, line?: number) => {
      const target = pane || refs.current.activePane;
      const sequence = (fileOpens.current.get(target) || 0) + 1;
      fileOpens.current.set(target, sequence);
      try {
        if (!refs.current.buffers[path]) {
          const buffer = await rpc<Buffer>("read_file", {
            project: project.id,
            path,
          });
          if (!mounted.current) return false;
          setBuffers((b) => (b[path] ? b : { ...b, [path]: buffer }));
        }
        if (fileOpens.current.get(target) !== sequence) return false;
        setLayout((current) =>
          update(current, target, (p) => ({
            ...p,
            active: path,
            tabs: p.tabs.includes(path) ? p.tabs : [...p.tabs, path],
          })),
        );
        setActivePane(target);
        if (line) setReveal({ file: path, line, nonce: Date.now() });
        return true;
      } catch (e) {
        if (mounted.current && fileOpens.current.get(target) === sequence)
          onError(String(e));
        return false;
      }
    },
    [project.id],
  );
  useEffect(() => {
    mounted.current = true;
    void refresh();
    void rpc<{ buffers: Record<string, Buffer>; warnings: string[] }>(
      "read_drafts",
      {
        project: project.id,
      },
    )
      .then(async ({ buffers: drafts, warnings }) => {
        if (!mounted.current) return;
        const recovered = Object.fromEntries(
          Object.entries(drafts).map(([k, v]) => [
            k,
            { ...v, dirty: true, recovered: true },
          ]),
        );
        refs.current.buffers = recovered;
        setBuffers(recovered);
        if (warnings.length) onError(warnings.join("\n"));
        draftsHydrated.current = true;
        setDraftsLoaded(true);
        for (const path of Object.keys(recovered)) await open(path);
        await open(project.main);
      })
      .catch(fail);
    return () => {
      mounted.current = false;
      ticket.current++;
      void rpc("cancel", { key: `${project.id}:caret` });
      if (draftsHydrated.current)
        void rpc("save_drafts", {
          project: project.id,
          value: Object.fromEntries(
            Object.entries(refs.current.buffers).filter(([, b]) => b.dirty),
          ),
        });
    };
  }, [project.id]);
  useEffect(() => {
    let stale = false;
    if (navigate)
      void open(navigate.path, undefined, navigate.line).then((opened) => {
        if (opened && mounted.current && !stale)
          setReveal({ ...navigate, file: navigate.path });
      });
    return () => {
      stale = true;
    };
  }, [navigate]);
  useEffect(() => {
    if (!draftsLoaded) return;
    onDrafts(sources);
    const timer = setTimeout(() => {
      void rpc("save_drafts", {
        project: project.id,
        value: Object.fromEntries(
          Object.entries(buffers).filter(([, b]) => b.dirty),
        ),
      }).catch(fail);
      if (autoSave)
        Object.values(buffers)
          .filter((b) => b.dirty && !b.recovered && !b.error)
          .forEach((b) => void save(b.path));
    }, 700);
    setProjectCompletions(project.id, [
      ...completionSources.current,
      ...Object.values(buffers).map((b) => b.text),
    ]);
    return () => clearTimeout(timer);
  }, [buffers, autoSave, draftsLoaded]);
  useEffect(() => {
    let stopped = false;
    const sourceFiles = tree.filter(
      (e) => !e.directory && /\.(tex|sty|cls|typ)$/.test(e.path),
    );
    void Promise.all(
      sourceFiles
        .slice(0, 200)
        .map((e) =>
          rpc<Buffer>("read_file", { project: project.id, path: e.path }).catch(
            () => null,
          ),
        ),
    ).then((all) => {
      if (!stopped)
        setProjectCompletions(project.id, [
          ...(completionSources.current = all
            .filter(Boolean)
            .map((b) => b!.text)),
          ...Object.values(refs.current.buffers).map((b) => b.text),
        ]);
    });
    return () => {
      stopped = true;
    };
  }, [tree]);
  useEffect(() => {
    const current = ++ticket.current;
    setPreview(null);
    void rpc("cancel", { key: `${project.id}:caret` });
    if (!caret) return;
    const timer = setTimeout(() => {
      setPreviewBusy(true);
      void rpc<Card | null>("source_preview", {
        project: project.id,
        key: `${project.id}:caret`,
        ticket: current,
        file: caret.file,
        offset: caret.offset,
        sources,
      })
        .then((card) => {
          if (ticket.current === current) setPreview(card);
        })
        .catch((e) => {
          if (ticket.current === current && String(e) !== "Cancelled")
            setPreview(null);
        })
        .finally(() => {
          if (ticket.current === current) setPreviewBusy(false);
        });
    }, 550);
    return () => clearTimeout(timer);
  }, [caret, buffers]);
  const split = (
    id: string,
    direction: "row" | "column",
    path?: string,
    before = false,
  ) => {
    setLayout((current) =>
      update(current, id, (p) => ({
        id: uid(),
        direction,
        ratio: 0.5,
        children: before
          ? [leaf(path || p.active), p]
          : [p, leaf(path || p.active)],
      })),
    );
  };
  const closeTab = (id: string, path: string) =>
    setLayout((current) =>
      update(current, id, (p) => {
        const tabs = p.tabs.filter((t) => t !== path);
        return {
          ...p,
          tabs,
          active:
            p.active === path
              ? tabs[Math.max(0, p.tabs.indexOf(path) - 1)] || ""
              : p.active,
        };
      }),
    );
  useEffect(() => {
    const handler = (e: Event) => {
      const command = (e as CustomEvent<string>).detail;
      const current = refs.current;
      const pane =
        leaves(current.layout).find((p) => p.id === current.activePane) ||
        leaves(current.layout)[0];
      if (command === "saveEquation")
        Object.keys(current.buffers).forEach((p) => void save(p));
      if (command === "nextSourceTab" || command === "previousSourceTab") {
        const i = pane.tabs.indexOf(pane.active);
        const path =
          pane.tabs[
            (i + (command === "nextSourceTab" ? 1 : -1) + pane.tabs.length) %
              pane.tabs.length
          ];
        if (path) void open(path, pane.id);
      }
      if (command === "splitSource") split(pane.id, "row");
      if (command === "splitSourceDown") split(pane.id, "column");
    };
    window.addEventListener("source-command", handler);
    return () => window.removeEventListener("source-command", handler);
  }, [save, open]);
  const change = (path: string, text: string) =>
    setBuffers((current) => {
      const b = current[path];
      if (!b || b.text === text) return current;
      return {
        ...current,
        [path]: { ...b, text, dirty: true, recovered: false, error: undefined },
      };
    });
  async function operation(action: string, path: string, directory: boolean) {
    try {
      if (action === "reveal") {
        await rpc("reveal", { project: project.id, path });
        return;
      }
      if (action === "copy" || action === "cut") {
        setClipboard({ path, cut: action === "cut" });
        return;
      }
      if (action === "copy-path") {
        await navigator.clipboard.writeText(project.root + "/" + path);
        return;
      }
      let destination = "";
      if (action === "create_file" || action === "create_folder") {
        const parent = directory
          ? path
          : path.split("/").slice(0, -1).join("/");
        const name = await ask(
          action === "create_file" ? "New File" : "New Folder",
          "",
          "Name",
        );
        if (!name) return;
        destination = [parent, name].filter(Boolean).join("/");
        await rpc("file_operation", {
          project: project.id,
          action,
          path: destination,
        });
        if (action === "create_file") await open(destination);
      } else if (action === "paste") {
        if (!clipboard) return;
        const parent = directory
          ? path
          : path.split("/").slice(0, -1).join("/");
        destination = [parent, clipboard.path.split("/").at(-1)]
          .filter(Boolean)
          .join("/");
        await rpc("file_operation", {
          project: project.id,
          action: clipboard.cut ? "move" : "copy",
          path: clipboard.path,
          destination,
        });
        if (clipboard.cut) {
          remap(clipboard.path, destination);
          setClipboard(null);
        }
      } else if (action === "rename" || action === "duplicate") {
        const name = await ask(
          action === "rename" ? "Rename" : "Duplicate",
          path,
          "Project-relative path",
        );
        if (!name || name === path) return;
        destination = name;
        await rpc("file_operation", {
          project: project.id,
          action: action === "duplicate" ? "copy" : "rename",
          path,
          destination,
        });
        if (action === "rename") remap(path, destination);
      } else if (action === "trash") {
        await rpc("file_operation", { project: project.id, action, path });
        setBuffers((current) =>
          Object.fromEntries(
            Object.entries(current).filter(
              ([name]) => name !== path && !name.startsWith(path + "/"),
            ),
          ),
        );
        for (const pane of leaves(refs.current.layout))
          for (const tab of pane.tabs)
            if (tab === path || tab.startsWith(path + "/"))
              closeTab(pane.id, tab);
      } else if (action === "reload") {
        const b = await rpc<Buffer>("read_file", { project: project.id, path });
        if (refs.current.buffers[path]?.dirty) {
          const answer = await ask(
            "Reload and replace the unsaved draft?",
            "Keep draft",
            "Type Reload to replace",
          );
          if (answer !== "Reload") return;
        }
        setBuffers((bs) => ({ ...bs, [path]: b }));
      }
      if (!["reload", "copy-path", "copy", "cut", "reveal"].includes(action))
        savedCallback.current();
      void refresh();
    } catch (e) {
      fail(e);
    }
  }
  function remap(from: string, to: string) {
    const rename = (p: string) =>
      p === from || p.startsWith(from + "/") ? to + p.slice(from.length) : p;
    setBuffers((bs) =>
      Object.fromEntries(
        Object.entries(bs).map(([p, b]) => [
          rename(p),
          { ...b, path: rename(p) },
        ]),
      ),
    );
    setLayout((current) => {
      const walk = (p: Pane): Pane =>
        "children" in p
          ? { ...p, children: p.children.map(walk) as [Pane, Pane] }
          : { ...p, tabs: p.tabs.map(rename), active: rename(p.active) };
      return walk(current);
    });
  }
  async function dragPreview(card: Card) {
    try {
      const path = await rpc<string>("preview_export", {
        project: project.id,
        card,
        format: project.exportFormat || "svg",
        fontSize: previewSize,
      });
      await startDrag({ item: [path], icon: card.pngPath!, mode: "copy" });
    } catch (e) {
      fail(e);
    }
  }
  const currentPath =
    leaves(layout).find((p) => p.id === activePane)?.active || "";
  const headings = (() => {
    const text = buffers[currentPath]?.text || "";
    return text.split("\n").flatMap((line, i) => {
      const m =
        project.format === "typst"
          ? line.match(/^(={1,6})\s+(.+)/)
          : line.match(
              /\\(section|subsection|subsubsection|chapter|part)\*?(?:\[[^\]]*\])?\{([^}]+)\}/,
            );
      return m
        ? [
            {
              line: i + 1,
              title: m[2].replace(/[{}]/g, ""),
              level:
                project.format === "typst"
                  ? m[1].length - 1
                  : Math.max(
                      0,
                      ["section", "subsection", "subsubsection"].indexOf(m[1]),
                    ),
            },
          ]
        : [];
    });
  })();
  const sidebar = (
    <div
      ref={navigationElement}
      className={`source-navigation tree-mode-${treeMode} ${compactNavigation ? "compact" : ""}`}
    >
      <div
        className="tree-tools compact-tree-tabs"
        role="tablist"
        aria-label="Source navigation"
      >
        <button
          role="tab"
          aria-selected={treeMode === "files"}
          className={treeMode === "files" ? "active" : ""}
          onClick={() => setTreeMode("files")}
        >
          Files
        </button>
        <button
          role="tab"
          aria-selected={treeMode === "outline"}
          className={treeMode === "outline" ? "active" : ""}
          onClick={() => setTreeMode("outline")}
        >
          Outline
        </button>
      </div>
      <section
        className="source-navigation-panel source-files"
        aria-label="Files"
      >
        <header className="tree-panel-heading">
          <strong>Files</strong>
          <span className="spacer" />
          <button
            title="New file"
            onClick={() => void operation("create_file", "", true)}
          >
            <Plus size={15} />
          </button>
        </header>
        <div className="tree-search">
          <Search size={14} />
          <input
            aria-label="Filter files"
            placeholder="Filter files"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        </div>
        <div
          className="file-tree"
          onContextMenu={(e) => {
            if (e.target === e.currentTarget) {
              e.preventDefault();
              setMenu({
                x: e.clientX,
                y: e.clientY,
                path: "",
                directory: true,
              });
            }
          }}
        >
          {tree
            .filter((e) =>
              query
                ? e.path.toLowerCase().includes(query.toLowerCase())
                : e.path
                    .split("/")
                    .slice(0, -1)
                    .every((_, i, a) =>
                      expanded.has(a.slice(0, i + 1).join("/")),
                    ),
            )
            .map((entry) => (
              <button
                key={entry.path}
                className={`tree-row ${currentPath === entry.path ? "selected" : ""}`}
                style={{
                  paddingLeft: 12 + entry.path.split("/").length * 12,
                }}
                title={entry.path}
                onClick={() => {
                  if (entry.directory)
                    setExpanded((old) => {
                      const n = new Set(old);
                      n.has(entry.path)
                        ? n.delete(entry.path)
                        : n.add(entry.path);
                      return n;
                    });
                  else void open(entry.path);
                }}
                onContextMenu={(e) => {
                  e.preventDefault();
                  setMenu({ x: e.clientX, y: e.clientY, ...entry });
                }}
              >
                {entry.directory ? (
                  expanded.has(entry.path) ? (
                    <ChevronDown size={13} />
                  ) : (
                    <ChevronRight size={13} />
                  )
                ) : (
                  <File size={13} />
                )}
                <span>{entry.path.split("/").at(-1)}</span>
                {buffers[entry.path]?.dirty && (
                  <span className="dirty-dot">●</span>
                )}
              </button>
            ))}
        </div>
      </section>
      <section
        className="source-navigation-panel source-outline"
        aria-label="Outline"
      >
        <header className="tree-panel-heading">
          <strong>Outline</strong>
        </header>
        <div className="file-tree">
          {headings.map((h) => (
            <button
              key={h.line}
              className="tree-row"
              style={{ paddingLeft: 16 + h.level * 14 }}
              onClick={() =>
                setReveal({
                  file: currentPath,
                  line: h.line,
                  nonce: Date.now(),
                })
              }
            >
              {h.title}
            </button>
          ))}
          {!headings.length && <div className="muted padded">No headings</div>}
        </div>
      </section>
    </div>
  );
  const host = document.getElementById("source-sidebar");
  return (
    <>
      {host && createPortal(sidebar, host)}
      <div className="source-workspace">
        <PaneView
          pane={layout}
          onRatio={(id, ratio) =>
            setLayout((current) => {
              const walk = (p: Pane): Pane =>
                "children" in p
                  ? {
                      ...p,
                      ratio: p.id === id ? ratio : p.ratio,
                      children: p.children.map(walk) as [Pane, Pane],
                    }
                  : p;
              return walk(current);
            })
          }
          renderLeaf={(p) => (
            <section
              key={p.id}
              className={`source-pane ${p.id === activePane ? "focused" : ""}`}
              onMouseDown={() => setActivePane(p.id)}
              onDragOver={(e) => {
                if (
                  !e.dataTransfer.types.includes("application/mathlidelux-tab")
                )
                  return;
                e.preventDefault();
                const r = e.currentTarget.getBoundingClientRect();
                const x = (e.clientX - r.left) / r.width,
                  y = (e.clientY - r.top) / r.height;
                setDrop({
                  pane: p.id,
                  edge:
                    x < 0.22
                      ? "left"
                      : x > 0.78
                        ? "right"
                        : y < 0.24
                          ? "top"
                          : y > 0.76
                            ? "bottom"
                            : "center",
                });
              }}
              onDragLeave={(e) => {
                if (!e.currentTarget.contains(e.relatedTarget as Node))
                  setDrop(null);
              }}
              onDrop={(e) => {
                e.preventDefault();
                try {
                  const from = JSON.parse(
                    e.dataTransfer.getData("application/mathlidelux-tab"),
                  );
                  if (drop && drop.edge !== "center") {
                    split(
                      p.id,
                      ["left", "right"].includes(drop.edge) ? "row" : "column",
                      from.path,
                      ["left", "top"].includes(drop.edge),
                    );
                  } else void open(from.path, p.id);
                } catch {}
                setDrop(null);
              }}
            >
              <div className="file-tabs">
                {p.tabs.map((path) => (
                  <div
                    role="tab"
                    aria-selected={p.active === path}
                    tabIndex={0}
                    key={path}
                    className={`file-tab ${p.active === path ? "active" : ""}`}
                    title={path}
                    draggable
                    onDragStart={(e) => {
                      e.dataTransfer.setData(
                        "application/mathlidelux-tab",
                        JSON.stringify({ path, pane: p.id }),
                      );
                      e.dataTransfer.effectAllowed = "copyMove";
                    }}
                    onClick={() => void open(path, p.id)}
                    onAuxClick={(e) => {
                      if (e.button === 1) {
                        e.preventDefault();
                        closeTab(p.id, path);
                      }
                    }}
                  >
                    <span>
                      {path.split("/").at(-1)}
                      {buffers[path]?.dirty ? " •" : ""}
                    </span>
                    <button
                      aria-label={`Close ${path}`}
                      onClick={(e) => {
                        e.stopPropagation();
                        closeTab(p.id, path);
                      }}
                    >
                      <X size={12} />
                    </button>
                  </div>
                ))}
                <span className="spacer" />
                <button
                  title="Split right (⌘\)"
                  onClick={() => split(p.id, "row")}
                >
                  <Columns2 size={15} />
                </button>
                <button
                  title="Split down (⌥⌘\)"
                  onClick={() => split(p.id, "column")}
                >
                  <Rows2 size={15} />
                </button>
                {leaves(layout).length > 1 && (
                  <button
                    title="Close pane"
                    onClick={() => {
                      const next = removePane(layout, p.id)!;
                      setLayout(next);
                      setActivePane(leaves(next)[0].id);
                    }}
                  >
                    <X size={15} />
                  </button>
                )}
              </div>
              {buffers[p.active]?.recovered && (
                <div className="inline-notice">
                  Recovered draft{" "}
                  <button onClick={() => void save(p.active)}>Save</button>
                </div>
              )}
              {buffers[p.active]?.error && (
                <div className="inline-notice error">
                  <TriangleAlert size={14} />
                  {buffers[p.active].error}
                  <button
                    onClick={() => void operation("reload", p.active, false)}
                  >
                    Reload…
                  </button>
                </div>
              )}
              {p.active && buffers[p.active] ? (
                <CodeEditor
                  project={project.id}
                  path={p.active}
                  value={buffers[p.active].text}
                  pane={p.id}
                  format={
                    p.active.endsWith(".typ")
                      ? "typst"
                      : /\.(tex|sty|cls)$/.test(p.active)
                        ? "latex"
                        : "text"
                  }
                  onChange={(text) => change(p.active, text)}
                  onCaret={(offset, line) => {
                    setCaret({ file: p.active, offset, line });
                    onCursor(p.active, offset, line);
                  }}
                  onFocus={() => setActivePane(p.id)}
                  onSave={() => void save(p.active)}
                  reveal={reveal?.file === p.active ? reveal : undefined}
                />
              ) : (
                <div className="empty">
                  <Folder size={32} />
                  <span>Open a file</span>
                </div>
              )}
              {drop?.pane === p.id && (
                <div className={`pane-drop ${drop.edge}`} />
              )}
            </section>
          )}
        />
        <footer className="source-status">
          <span title={currentPath}>{currentPath}</span>
          <span className="spacer" />
          {caret && <span>Ln {caret.line}</span>}
          <span>{autoSave ? "Auto Save" : "Manual Save"}</span>
          {previewBusy && <RefreshCw size={12} className="spin" />}
        </footer>
        {preview && (
          <div className="caret-preview">
            <header>
              <span>{preview.kind === "figure" ? "Figure" : "Preview"}</span>
              <span className="spacer" />
              <select
                title="Output font size"
                value={previewSize}
                onChange={(e) => setPreviewSize(Number(e.target.value))}
              >
                {[0, 12, 16, 20, 24, 32, 40, 48, 60, 72].map((n) => (
                  <option value={n} key={n}>
                    {n ? n + " pt" : "Original"}
                  </option>
                ))}
              </select>
              <button
                title="Copy image"
                disabled={preview.status !== "ready"}
                onClick={() =>
                  void rpc<string>("preview_export", {
                    project: project.id,
                    card: preview,
                    format: project.exportFormat || "svg",
                    fontSize: previewSize,
                  })
                    .then((path) => rpc("copy_images", { paths: [path] }))
                    .catch(fail)
                }
              >
                <ClipboardCopy size={15} />
              </button>
              <button title="Close preview" onClick={() => setPreview(null)}>
                <X size={15} />
              </button>
            </header>
            {preview.status === "failed" ? (
              <pre className="preview-error">{preview.error}</pre>
            ) : (
              <ImagePreview
                card={preview}
                onDrag={(card) => void dragPreview(card)}
              />
            )}
          </div>
        )}
      </div>
      {menu && (
        <ContextMenu
          x={menu.x}
          y={menu.y}
          onClose={() => setMenu(null)}
          items={[
            {
              label: "New File…",
              action: () =>
                void operation("create_file", menu.path, menu.directory),
            },
            {
              label: "New Folder…",
              action: () =>
                void operation("create_folder", menu.path, menu.directory),
            },
            ...(menu.path
              ? [
                  {
                    label: "Rename…",
                    action: () =>
                      void operation("rename", menu.path, menu.directory),
                  },
                  {
                    label: "Duplicate…",
                    action: () =>
                      void operation("duplicate", menu.path, menu.directory),
                  },
                  {
                    label: "Copy",
                    action: () =>
                      void operation("copy", menu.path, menu.directory),
                  },
                  {
                    label: "Cut",
                    action: () =>
                      void operation("cut", menu.path, menu.directory),
                  },
                ]
              : []),
            {
              label: "Paste",
              disabled: !clipboard,
              action: () => void operation("paste", menu.path, menu.directory),
            },
            {
              label: "Reveal in Finder",
              action: () => void operation("reveal", menu.path, menu.directory),
            },
            {
              label: "Copy Path",
              action: () =>
                void operation("copy-path", menu.path, menu.directory),
            },
            ...(menu.path
              ? [
                  {
                    label: "Move to Trash",
                    action: () =>
                      void operation("trash", menu.path, menu.directory),
                  },
                ]
              : []),
          ]}
        />
      )}
    </>
  );
}
