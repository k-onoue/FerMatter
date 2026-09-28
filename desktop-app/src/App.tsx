import { dropSVG } from "./svgImport";
import { LibraryNavigation } from "./LibraryNavigation";
import { VariantEditor } from "./VariantEditor";
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useDesktopBackground } from "./useDesktopBackground";
import { open, save } from "@tauri-apps/plugin-dialog";
import { startDrag, useCardArtifacts } from "./artifactLease";
import {
  ArrowLeft,
  ClipboardCopy,
  Code2,
  Download,
  FileText,
  FolderOpen,
  Grid2X2,
  LoaderCircle,
  PanelLeftClose,
  Pin,
  Plus,
  RefreshCw,
  Search,
  Settings2,
  SunMoon,
  X,
  TriangleAlert,
  Zap,
} from "lucide-react";
import { rpc, uid, projectName } from "./api";
import { setProjectCompletions, disposeProjectModels } from "./editor";
import {
  SnapshotWriter,
  registerWindowFlush,
  flushWindow,
  lockWindowInteraction,
} from "./persistence";
import { installWindowPersistence } from "./windowPersistence";
import { mergeScan, mergeRenderedCard } from "./projectUpdates";
import { SourceWorkspace, type SourceSession } from "./SourceWorkspace";
import { PdfPane, type PdfViewState } from "./PdfPane";
import { WorkspaceArtifacts, WorkspaceTabs } from "./WorkspaceTabs";
import { newWorkspace, type WorkspaceState } from "./workspaces";
import { restoredVariant } from "./restoredVariant";
import type { InstantDraft } from "./instant";
import { OutputSizeOptions } from "./OutputSizeOptions";
import { ImagePreview, Modal, Prompt, ContextMenu, type Ask } from "./ui";
import { Settings } from "./settings";
import { defaults } from "./appearance";
import { useSettingsDocument } from "./useSettingsDocument";
import type { SettingsSnapshot } from "./settingsPersistence";
import { useAppearance } from "./useAppearance";
import { commandForEvent, validateBindings } from "./shortcuts";
import { stepZoom } from "./scrollZoom";
import { useControlScroll } from "./useControlScroll";
import type {
  Project,
  Card,
  Preferences,
  Theme,
  Keybindings,
  DocumentPDF,
  Buffer,
  TreeEntry,
  SourceTarget,
} from "./types";
import { targetAtSource, targetsAtLine } from "./navigation";
import { ProjectsPage } from "./ProjectsPage";
import { ProjectSetup } from "./ProjectSetup";
import { ProjectRecovery, type ProjectProblem } from "./ProjectRecovery";
const initialPreferences: Preferences = {
  appearance: "dark",
  autoSave: true,
  opacity: 0.88,
  theme: defaults.id,
  pdfWidth: 420,
  projectView: "grid",
};
const initialKeys: Keybindings = {
  schemaVersion: 1,
  kind: "mathlidelux.keybindings",
  bindings: {},
};
type PromptState = {
  title: string;
  value: string;
  label: string;
  resolve: (value: string | null) => void;
};
import { NewProjectDialog } from "./NewProjectDialog";
import { savedVariant } from "./variants";
import { initialTemplates, type TemplateConfiguration } from "./templates";
export default function App() {
  const [projects, setProjects] = useState<Project[]>([]),
    [project, setProject] = useState<Project | null>(null);
  const [workspaces, setWorkspaceState] = useState<WorkspaceState[]>([]);
  const workspacesRef = useRef(workspaces);
  const sourceSessions = useRef(new Map<string, SourceSession>());
  const pdfViews = useRef(new Map<string, PdfViewState>());
  const workspaceRegistration = useRef(Promise.resolve());
  const [projectProblems, setProjectProblems] = useState<ProjectProblem[]>([]);
  const [mode, setMode] = useState<"library" | "source">("library");
  const [libraryStale, setLibraryStale] = useState(false);
  const [sourceMounted, setSourceMounted] = useState(false);
  const [settings, setSettings] = useState<Preferences>(initialPreferences),
    [themes, setThemes] = useState<Theme[]>([]),
    [theme, setTheme] = useState<Theme>(defaults),
    [keys, setKeys] = useState<Keybindings>(initialKeys);
  const preferencesDocument = useSettingsDocument(
    "preferences.json",
    initialPreferences,
    setSettings,
  );
  const themeDocument = useSettingsDocument(
    "theme.json",
    defaults,
    setTheme,
    false,
  );
  const keysDocument = useSettingsDocument(
    "keybindings.json",
    initialKeys,
    setKeys,
  );
  const [systemDark, setSystemDark] = useState(
    matchMedia("(prefers-color-scheme: dark)").matches,
  );
  const [showSettings, setShowSettings] = useState(false);
  const [showProjectSettings, setShowProjectSettings] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [windowZoom, setWindowZoom] = useState(1);
  const [defaultProjectDirectory, setDefaultProjectDirectory] = useState("");
  const [setupRequired, setSetupRequired] = useState(true);
  const [canvasZoom, setCanvasZoom] = useState(1);
  const canvasScroll = useControlScroll((steps) =>
    setCanvasZoom((zoom) => stepZoom(zoom, steps)),
  );
  const [search, setSearch] = useState("");
  const [kind, setKind] = useState("all");
  const [filter, setFilter] = useState("all");
  const [collapsed, setCollapsed] = useState(new Set<string>());
  const [selected, setSelected] = useState(new Set<string>());
  const [busy, setBusy] = useState("");
  const [pin, setPin] = useState(false);
  const [pdfOpen, setPdfOpen] = useState(false),
    [pdf, setPdf] = useState<DocumentPDF | null>(null),
    [pdfBusy, setPdfBusy] = useState(false);
  const [pdfTarget, setPdfTarget] = useState("");
  const [navigate, setNavigate] = useState<{
    path: string;
    line: number;
    nonce: number;
    startUTF16?: number;
    endUTF16?: number;
  }>();
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [editing, setEditingState] = useState<Card | null>(null);
  const [editPreview, setEditPreview] = useState<Card | null>(null);
  const [editBusy, setEditBusy] = useState(false);
  useCardArtifacts(editPreview);
  const [newProject, setNewProject] = useState(false);
  const [templates, setTemplates] =
    useState<TemplateConfiguration>(initialTemplates);
  const [prompt, setPrompt] = useState<PromptState | null>(null);
  const [context, setContext] = useState<{
    x: number;
    y: number;
    card: Card;
  } | null>(null);
  const [marquee, setMarquee] = useState<{
    x: number;
    y: number;
    width: number;
    height: number;
  } | null>(null);
  const [inspector, setInspector] = useState(false);
  const [ready, setReady] = useState(false);
  const projectRef = useRef(project);
  projectRef.current = project;
  const writer = useRef(
    new SnapshotWriter<Project>((next) =>
      rpc("save_project", { project: next.id, value: next }),
    ),
  );
  const editingRef = useRef(editing);
  editingRef.current = editing;
  function setEditing(value: Card | null) {
    // Older restored drafts waited for an explicit Render. Opening the editor
    // now previews them automatically, without rendering unopened saved cards.
    if (value?.requiresRender === true)
      value = { ...value, requiresRender: false };
    editingRef.current = value;
    setEditingState(value);
  }
  const closeFlush = useRef(async () => {});
  const projectTransition = useRef(0);
  const jobs = useRef(new Map<string, string>());
  const editJob = useRef(0);
  const selectedAnchor = useRef<string | undefined>(undefined);
  const grid = useRef<HTMLDivElement>(null);
  const searchInput = useRef<HTMLInputElement>(null);
  const viewRef = useRef<WorkspaceState | null>(null);
  if (project)
    viewRef.current = {
      project,
      mode,
      libraryStale,
      sourceMounted,
      canvasZoom,
      search,
      kind,
      filter,
      collapsed,
      selected,
      selectedAnchor: selectedAnchor.current,
      pdfOpen,
      pdf,
      pdfTarget,
      pdfView: pdfViews.current.get(project.id),
      navigate,
      drafts,
      editing,
      editPreview,
      inspector,
      scrollTop: grid.current?.scrollTop || 0,
    };
  const importFlight = useRef<Promise<void> | null>(null);
  const importCards = useRef(new Map<string, Card>());
  const consumeImports = useRef<() => Promise<void>>(async () => {});
  const dark =
    settings.appearance === "system"
      ? systemDark
      : settings.appearance === "dark";
  useAppearance(theme, dark, settings.opacity);
  const ask: Ask = (title, value = "", label = "Name") =>
    new Promise((resolve) => setPrompt({ title, value, label, resolve }));
  function report(e: unknown) {
    if (String(e) !== "Cancelled") setError(String(e));
    if (String(e).includes("File:")) {
      void rpc<ProjectProblem[]>("project_problems")
        .then(setProjectProblems)
        .catch(() => {});
    }
  }
  function setWorkspaces(next: WorkspaceState[]) {
    workspacesRef.current = next;
    setWorkspaceState(next);
  }
  function registerWorkspaces() {
    const projects = workspacesRef.current.map(
      (workspace) => workspace.project.id,
    );
    const operation = workspaceRegistration.current
      .catch(() => {})
      .then(async () => {
        await rpc("resident_workspace", { projects });
      });
    workspaceRegistration.current = operation;
    return operation;
  }
  function captureWorkspace() {
    const current = projectRef.current;
    if (!current || !viewRef.current) return;
    const snapshot: WorkspaceState = {
      ...viewRef.current,
      project: current,
      editing: editingRef.current,
      selectedAnchor: selectedAnchor.current,
      pdfView: pdfViews.current.get(current.id),
      scrollTop: grid.current?.scrollTop || 0,
    };
    setWorkspaces(
      workspacesRef.current.map((item) =>
        item.project.id === current.id ? snapshot : item,
      ),
    );
  }
  function restoreWorkspace(next: WorkspaceState) {
    viewRef.current = next;
    projectRef.current = next.project;
    setProject(next.project);
    setMode(next.mode);
    setLibraryStale(next.libraryStale);
    setSourceMounted(next.sourceMounted);
    setCanvasZoom(next.canvasZoom);
    setSearch(next.search);
    setKind(next.kind);
    setFilter(next.filter);
    setCollapsed(next.collapsed);
    setSelected(next.selected);
    selectedAnchor.current = next.selectedAnchor;
    setPdfOpen(next.pdfOpen);
    setPdf(next.pdf);
    setPdfTarget(next.pdfTarget);
    setNavigate(next.navigate);
    setDrafts(next.drafts);
    setEditing(next.editing);
    setEditPreview(next.editPreview);
    setEditBusy(false);
    setInspector(next.inspector);
    setError("");
    setContext(null);
    setShowProjectSettings(false);
  }
  useLayoutEffect(() => {
    if (grid.current)
      grid.current.scrollTop =
        workspacesRef.current.find((item) => item.project.id === project?.id)
          ?.scrollTop || 0;
  }, [project?.id]);
  useEffect(() => {
    void getCurrentWebview().setZoom(windowZoom).catch(report);
  }, [windowZoom]);
  useEffect(() => {
    void rpc<{
      projects: Project[];
      projectProblems: ProjectProblem[];
      setupRequired: boolean;
      defaultProjectDirectory: string;
      settings: SettingsSnapshot<Preferences>;
      themes: Theme[];
      theme: SettingsSnapshot<Theme>;
      keybindings: SettingsSnapshot<Keybindings>;
      templates: TemplateConfiguration;
    }>("startup")
      .then((data) => {
        setProjects(data.projects);
        setProjectProblems(data.projectProblems);
        setSetupRequired(data.setupRequired);
        setDefaultProjectDirectory(data.defaultProjectDirectory);
        preferencesDocument.accept({
          ...data.settings,
          value: { ...initialPreferences, ...data.settings.value },
        });
        if (data.settings.value.pinned === true) {
          void getCurrentWindow()
            .setAlwaysOnTop(true)
            .then(() => setPin(true))
            .catch(report);
        }
        setThemes(data.themes);
        themeDocument.accept(data.theme);
        try {
          validateBindings(data.keybindings.value);
          keysDocument.accept(data.keybindings);
        } catch (e) {
          report(e);
        }
        setTemplates(
          Array.isArray(data.templates?.custom)
            ? data.templates
            : initialTemplates,
        );
        setReady(true);
      })
      .catch(report);
    const q = matchMedia("(prefers-color-scheme: dark)");
    const change = () => setSystemDark(q.matches);
    q.addEventListener("change", change);
    return () => q.removeEventListener("change", change);
  }, []);
  useEffect(() => {
    let gone = false;
    let cleanup: (() => void) | undefined;
    void listen<{
      ticket: string;
      project: string;
      card: Partial<Card> & { id: string };
    }>("render-progress", (event) => {
      const e = event.payload;
      if (
        jobs.current.get("render") !== e.ticket ||
        projectRef.current?.id !== e.project
      )
        return;
      setProject((p) =>
        p && p.id === e.project
          ? {
              ...p,
              cards: p.cards.map((c) =>
                c.id === e.card.id ? mergeRenderedCard(c, e.card) : c,
              ),
            }
          : p,
      );
    }).then((fn) => {
      if (gone) fn();
      else cleanup = fn;
    });
    return () => {
      gone = true;
      cleanup?.();
    };
  }, []);
  useEffect(() => {
    if (!notice) return;
    const t = setTimeout(() => setNotice(""), 2500);
    return () => clearTimeout(t);
  }, [notice]);
  function persist(next: Project) {
    if (projectRef.current?.id === next.id) {
      setProject(next);
      projectRef.current = next;
    }
    setWorkspaces(
      workspacesRef.current.map((item) =>
        item.project.id === next.id ? { ...item, project: next } : item,
      ),
    );
    setProjects((all) => [next, ...all.filter((p) => p.id !== next.id)]);
    const operation = writer.current.save(next);
    void operation.catch(report);
    return operation;
  }
  closeFlush.current = async () => {
    const current = projectRef.current;
    if (current) {
      let draft;
      do {
        draft = editingRef.current;
        if (draft)
          await persist({ ...projectRef.current!, libraryDraft: draft });
      } while (draft !== editingRef.current);
    }
    await Promise.all([
      writer.current.flush(),
      preferencesDocument.flush(),
      themeDocument.flush(),
      keysDocument.flush(),
    ]);
  };
  useEffect(() => {
    const unregister = registerWindowFlush("library", () =>
      closeFlush.current(),
    );
    let gone = false;
    let cleanup: (() => void) | undefined;
    void installWindowPersistence(report)
      .then((stop) => {
        if (gone) stop();
        else cleanup = stop;
      })
      .catch(report);
    return () => {
      gone = true;
      cleanup?.();
      unregister();
    };
  }, []);
  async function updateSettings(next: Partial<Preferences>) {
    try {
      await preferencesDocument.patch(next);
    } catch (e) {
      report(e);
    }
  }
  useEffect(() => {
    if (!project) return;
    let stopped = false;
    const id = project.id;
    void rpc<TreeEntry[]>("tree", { project: id })
      .then(async (entries) => {
        const sources = entries
          .filter((e) => !e.directory && /\.(tex|sty|cls|typ)$/.test(e.path))
          .slice(0, 200);
        const buffers = await Promise.all(
          sources.map((e) =>
            rpc<Buffer>("read_file", { project: id, path: e.path }).catch(
              () => null,
            ),
          ),
        );
        if (!stopped)
          setProjectCompletions(
            id,
            buffers.filter(Boolean).map((b) => b!.text),
          );
      })
      .catch(() => {});
    return () => {
      stopped = true;
    };
  }, [project?.id]);
  async function updateTheme(next: Theme) {
    try {
      await themeDocument.replace(next);
    } catch (e) {
      report(e);
    }
  }
  async function updateKeys(next: Keybindings) {
    try {
      validateBindings(next);
      await keysDocument.replace(next);
    } catch (e) {
      report(e);
    }
  }
  async function cancelJobs() {
    const p = projectRef.current;
    for (const channel of ["render", "scan", "document", "edit", "caret"])
      if (p) void rpc("cancel", { key: `${p.id}:${channel}` });
    jobs.current.clear();
    editJob.current++;
    setBusy("");
    setPdfBusy(false);
  }
  async function activate(
    p: Project,
    transition = ++projectTransition.current,
    importing = false,
    refreshEntrypoint = false,
  ) {
    if (transition !== projectTransition.current) return false;
    const release = lockWindowInteraction();
    try {
      await flushWindow(importing ? ["workspace-imports"] : []);
      if (transition !== projectTransition.current) return false;
      await cancelJobs();
      if (transition !== projectTransition.current) return false;
      captureWorkspace();
      const existing = workspacesRef.current.find(
        (item) => item.project.id === p.id,
      );
      let next = existing || newWorkspace(p);
      if (existing && refreshEntrypoint) {
        const changed =
          existing.project.main !== p.main ||
          existing.project.format !== p.format;
        next = {
          ...existing,
          project: {
            ...existing.project,
            main: p.main,
            format: p.format,
            candidates: p.candidates,
          },
          libraryStale: existing.libraryStale || changed,
          pdf: changed ? null : existing.pdf,
        };
        setWorkspaces(
          workspacesRef.current.map((item) =>
            item.project.id === p.id ? next : item,
          ),
        );
      } else if (!existing) setWorkspaces([...workspacesRef.current, next]);
      // open_project saves its chosen entrypoint before this transition flushes
      // the old workspace. Restore the chosen metadata before a rescan reads it.
      if (existing && refreshEntrypoint) await persist(next.project);
      if (transition !== projectTransition.current) return false;
      restoreWorkspace(next);
      setProjects((all) => [next.project, ...all.filter((x) => x.id !== p.id)]);
      await registerWorkspaces();
      if (!existing && !p.cards?.length && !importing) void rescan(p);
      return true;
    } finally {
      release();
    }
  }
  async function showProjects() {
    const transition = ++projectTransition.current;
    const release = lockWindowInteraction();
    try {
      await flushWindow();
      if (transition !== projectTransition.current) return;
      await cancelJobs();
      if (transition !== projectTransition.current) return;
      captureWorkspace();
      projectRef.current = null;
      viewRef.current = null;
      setProject(null);
      setEditing(null);
      setEditPreview(null);
      setSourceMounted(false);
      setPdf(null);
      setPdfOpen(false);
      setContext(null);
      setShowProjectSettings(false);
    } finally {
      release();
    }
  }
  async function selectWorkspace(id: string | null) {
    if (id === projectRef.current?.id || (!id && !projectRef.current)) return;
    if (!id) return showProjects();
    const target = workspacesRef.current.find((item) => item.project.id === id);
    if (target) await activate(target.project);
  }
  async function closeWorkspace(id: string) {
    const release = lockWindowInteraction();
    try {
      await flushWindow();
      const all = workspacesRef.current;
      const index = all.findIndex((item) => item.project.id === id);
      if (index < 0) return;
      if (projectRef.current?.id === id) {
        const next = all[index + 1] || all[index - 1];
        if (next) await activate(next.project);
        else await showProjects();
        if (projectRef.current?.id === id) return;
      }
      setWorkspaces(
        workspacesRef.current.filter((item) => item.project.id !== id),
      );
      sourceSessions.current.delete(id);
      pdfViews.current.delete(id);
      disposeProjectModels(id);
      await registerWorkspaces();
    } finally {
      release();
    }
  }
  async function reconcileProjects(next: Project[]) {
    const release = lockWindowInteraction();
    try {
      await flushWindow();
      const ids = new Set(next.map((item) => item.id));
      if (projectRef.current && !ids.has(projectRef.current.id))
        await showProjects();
      const removed = workspacesRef.current.filter(
        (item) => !ids.has(item.project.id),
      );
      setWorkspaces(
        workspacesRef.current
          .filter((item) => ids.has(item.project.id))
          .map((item) => ({
            ...item,
            project:
              next.find((project) => project.id === item.project.id) ||
              item.project,
          })),
      );
      for (const item of removed) {
        sourceSessions.current.delete(item.project.id);
        pdfViews.current.delete(item.project.id);
        disposeProjectModels(item.project.id);
      }
      setProjects(next);
      await registerWorkspaces();
    } finally {
      release();
    }
  }
  consumeImports.current = () => {
    if (importFlight.current) return importFlight.current;
    if (!ready || setupRequired) return Promise.resolve();
    const operation = (async () => {
      const release = lockWindowInteraction();
      try {
        let item = await rpc<{
          token: string;
          project: Project;
          draft: InstantDraft;
        } | null>("incoming_import");
        while (item) {
          if (
            !(await activate(item.project, ++projectTransition.current, true))
          )
            throw new Error("SVG is waiting. Select its project to try again.");
          const current = projectRef.current!;
          let restored = importCards.current.get(item.token);
          if (!restored) {
            restored = restoredVariant(item.draft, current);
            importCards.current.set(item.token, restored);
          }
          // A second import must never replace the user's existing scratch work.
          const previous = editingRef.current;
          const preservedID = `draft-before-${item.token}`;
          const cards =
            previous?.tex.trim() &&
            previous.id !== restored.id &&
            !current.cards.some((card) => card.id === preservedID)
              ? [
                  ...current.cards,
                  {
                    ...savedVariant(
                      previous,
                      viewRef.current?.editPreview || null,
                    ),
                    id: preservedID,
                  },
                ]
              : current.cards;
          await persist({ ...current, cards, libraryDraft: restored });
          setEditing(restored);
          setEditPreview(null);
          setMode("library");
          setEditBusy(false);
          const token = item.token;
          item = await rpc("incoming_import", { token });
          importCards.current.delete(token);
        }
      } finally {
        release();
      }
    })();
    importFlight.current = operation;
    void operation
      .finally(() => {
        if (importFlight.current === operation) importFlight.current = null;
      })
      .catch(() => {});
    return operation;
  };
  useEffect(() => {
    const unregister = registerWindowFlush("workspace-imports", () =>
      consumeImports.current(),
    );
    let stopped = false;
    const consume = () => {
      if (!stopped) void consumeImports.current().catch(report);
    };
    const subscription = listen("workspace-import-pending", consume);
    window.addEventListener("focus", consume);
    void subscription.then(consume).catch(report);
    return () => {
      stopped = true;
      unregister();
      window.removeEventListener("focus", consume);
      void subscription.then((stop) => stop());
    };
  }, []);
  useEffect(() => {
    if (ready && !setupRequired) void consumeImports.current().catch(report);
  }, [ready, setupRequired]);
  async function openProject(type = "folder") {
    const transition = ++projectTransition.current;
    try {
      const path = await open({
        directory: type === "folder",
        multiple: false,
        filters:
          type === "zip"
            ? [{ name: "ZIP project", extensions: ["zip"] }]
            : type === "file"
              ? [{ name: "LaTeX or Typst", extensions: ["tex", "typ"] }]
              : undefined,
      });
      if (typeof path !== "string") return;
      await activate(
        await rpc<Project>("open_project", { path }),
        transition,
        false,
        true,
      );
    } catch (e) {
      report(e);
    }
  }
  async function openRecent(p: Project) {
    const transition = ++projectTransition.current;
    try {
      await activate(
        await rpc<Project>("open_project", { path: p.root, main: p.main }),
        transition,
        false,
        true,
      );
    } catch (e) {
      report(e);
    }
  }
  async function renderCards(p: Project, cards = p.cards) {
    cards = cards.filter((card) => card.requiresRender !== true);
    if (!cards.length) return;
    const ticket = uid();
    jobs.current.set("render", ticket);
    setBusy("Rendering…");
    try {
      await writer.current.flush();
      if (
        jobs.current.get("render") !== ticket ||
        projectRef.current?.id !== p.id
      )
        return;
      const results = await rpc<Partial<Card>[]>("render", {
        project: p.id,
        key: `${p.id}:render`,
        ticket,
        cards,
      });
      if (
        jobs.current.get("render") !== ticket ||
        projectRef.current?.id !== p.id
      )
        return;
      const current = projectRef.current;
      const byID = new Map(results.map((c) => [c.id, c]));
      await persist({
        ...current,
        cards: current.cards.map((card) =>
          mergeRenderedCard(card, byID.get(card.id)),
        ),
      });
    } catch (e) {
      if (jobs.current.get("render") === ticket) report(e);
    } finally {
      if (jobs.current.get("render") === ticket) {
        jobs.current.delete("render");
        setBusy("");
      }
    }
  }
  async function rescan(p = projectRef.current) {
    if (!p) return;
    await cancelJobs();
    const ticket = uid();
    jobs.current.set("scan", ticket);
    setBusy("Scanning…");
    try {
      await writer.current.flush();
      if (
        jobs.current.get("scan") !== ticket ||
        projectRef.current?.id !== p.id
      )
        return;
      const next = await rpc<Project>("scan", {
        project: p.id,
        key: `${p.id}:scan`,
        ticket,
      });
      if (
        jobs.current.get("scan") !== ticket ||
        projectRef.current?.id !== p.id
      )
        return;
      if (
        projectRef.current.main !== p.main ||
        projectRef.current.engine !== p.engine
      ) {
        void rescan(projectRef.current);
        return;
      }
      const merged = mergeScan(projectRef.current, next);
      await persist(merged);
      await renderCards(merged);
    } catch (e) {
      if (
        jobs.current.get("scan") === ticket &&
        projectRef.current?.id === p.id
      ) {
        report(e);
        setBusy("");
      }
    }
  }
  function closePdf() {
    jobs.current.delete("document");
    const id = projectRef.current?.id;
    if (id) void rpc("cancel", { key: `${id}:document` }).catch(report);
    setPdfOpen(false);
    setPdf(null);
    setPdfBusy(false);
  }
  async function buildPdf() {
    const p = projectRef.current;
    if (!p) return;
    const ticket = uid();
    jobs.current.set("document", ticket);
    setPdfBusy(true);
    setPdfOpen(true);
    try {
      await writer.current.flush();
      if (
        jobs.current.get("document") !== ticket ||
        projectRef.current?.id !== p.id
      )
        return;
      const result = await rpc<DocumentPDF>("document", {
        project: p.id,
        key: `${p.id}:document`,
        ticket,
        sources: drafts,
      });
      if (
        jobs.current.get("document") === ticket &&
        projectRef.current?.id === p.id
      )
        setPdf(result);
    } catch (e) {
      if (jobs.current.get("document") === ticket) report(e);
    } finally {
      if (jobs.current.get("document") === ticket) setPdfBusy(false);
    }
  }
  function switchMode(next: "library" | "source") {
    if (next === "source") setSourceMounted(true);
    setMode(next);
  }
  useEffect(() => {
    if (mode !== "library" || !libraryStale || !project) return;
    const timer = setTimeout(() => {
      setLibraryStale(false);
      void rescan();
    }, 200);
    return () => clearTimeout(timer);
  }, [mode, libraryStale, project?.id]);
  function showSource(card: Card) {
    if (!card.file) return;
    switchMode("source");
    setNavigate({ path: card.file, line: card.line || 1, nonce: Date.now() });
  }
  function editCard(card?: Card, figure = false) {
    if (!project) return;
    const source =
      card ||
      ({
        id: uid(),
        name: figure ? "New figure" : "New equation",
        tex: figure
          ? project.format === "typst"
            ? "#cetz.canvas({\n  import cetz.draw: *\n  line((0, 0), (2, 1))\n})"
            : "\\begin{tikzpicture}\n\\draw (0,0) -- (2,1);\n\\end{tikzpicture}"
          : project.format === "typst"
            ? "x^2 + y^2 = r^2"
            : "x^2 + y^2 = r^2",
        kind: figure ? "figure" : "block",
        origin: "custom",
        sourceOrder: project.cards.length,
        status: "pending",
      } as Card);
    setEditing({
      ...source,
      id: source.origin === "custom" ? source.id : "custom-" + uid(),
      origin: "custom",
      parentID: source.origin === "source" ? source.id : source.parentID,
      status: "pending",
      svgPath: undefined,
      pngPath: undefined,
      pdfPath: undefined,
    });
    setEditPreview(card || null);
  }
  useEffect(() => {
    const sequence = ++editJob.current;
    const p = project;
    if (!editing || !p || editing.requiresRender === true) {
      setEditBusy(false);
      return;
    }
    void rpc("cancel", { key: `${p.id}:edit` });
    const timer = setTimeout(() => {
      setEditBusy(true);
      void rpc<Partial<Card>[]>("render", {
        project: p.id,
        key: `${p.id}:edit`,
        ticket: uid(),
        cards: [editing],
      })
        .then((results) => {
          if (editJob.current === sequence && results[0])
            setEditPreview({ ...editing, ...results[0] });
        })
        .catch((e) => {
          if (editJob.current === sequence) report(e);
        })
        .finally(() => {
          if (editJob.current === sequence) setEditBusy(false);
        });
    }, 400);
    return () => clearTimeout(timer);
  }, [
    project?.id,
    editing?.tex,
    editing?.kind,
    editing?.id,
    editing?.requiresRender,
  ]);
  function closeEditor() {
    if (projectRef.current?.libraryDraft)
      void persist({ ...projectRef.current, libraryDraft: null });
    editingRef.current = null;
    editJob.current++;
    if (project) void rpc("cancel", { key: `${project.id}:edit` });
    setEditing(null);
    setEditPreview(null);
  }
  async function saveVariant() {
    if (!project || !editing) return;
    const draft = editing;
    const card = savedVariant(editing, editPreview);
    const exists = project.cards.some((c) => c.id === card.id);
    const next = {
      ...project,
      libraryDraft: null,
      cards: exists
        ? project.cards.map((c) => (c.id === card.id ? card : c))
        : [...project.cards, card],
    };
    await persist(next);
    if (projectRef.current?.id !== next.id || editingRef.current !== draft)
      return;
    setSelected(new Set([card.id]));
    closeEditor();
    if (card.status !== "ready") void renderCards(next, [card]);
  }
  async function exported(ids = [...selected]) {
    if (!project) throw new Error("Open a project.");
    if (!ids.length) throw new Error("Select an equation or figure.");
    await writer.current.flush();
    return rpc<string[]>("export", {
      project: project.id,
      ids,
      format: project.exportFormat || "svg",
    });
  }
  async function drag(card: Card) {
    try {
      const ids = selected.has(card.id) ? [...selected] : [card.id];
      const paths = await exported(ids);
      await startDrag({ item: paths, icon: card.pngPath!, mode: "copy" });
    } catch (e) {
      report(e);
    }
  }
  async function copyImages() {
    try {
      const paths = await exported();
      await rpc("copy_images", { paths });
      setNotice(paths.length > 1 ? `Copied ${paths.length} images` : "Copied");
    } catch (e) {
      report(e);
    }
  }
  async function exportImage() {
    if (!project) return;
    try {
      const ext =
        project.exportFormat === "jpeg" ? "jpg" : project.exportFormat || "svg";
      const dest = await save({
        defaultPath: `${project.cards.find((c) => selected.has(c.id))?.name || "Equation"}.${ext}`,
        filters: [{ name: ext.toUpperCase(), extensions: [ext] }],
      });
      if (!dest) return;
      await writer.current.flush();
      await rpc("export", {
        project: project.id,
        ids: [[...selected][0]],
        format: project.exportFormat || "svg",
        destination: dest,
      });
      setNotice("Exported");
    } catch (e) {
      report(e);
    }
  }
  const visible = useMemo(
    () =>
      project?.cards.filter(
        (c) =>
          (kind === "all" || c.kind === kind) &&
          (filter === "all" ||
            (filter === "unlabeled" && !project.sectionAssignments?.[c.id]) ||
            c.outlinePath?.includes(filter) ||
            project.sectionAssignments?.[c.id] === filter) &&
          (!search ||
            [c.name, c.tex, c.file].some((t) =>
              t?.toLowerCase().includes(search.toLowerCase()),
            )),
      ) || [],
    [project, kind, filter, search],
  );
  function choose(card: Card, e: React.MouseEvent) {
    if (e.shiftKey && selectedAnchor.current) {
      const a = visible.findIndex((c) => c.id === selectedAnchor.current),
        b = visible.findIndex((c) => c.id === card.id);
      setSelected(
        new Set(
          visible.slice(Math.min(a, b), Math.max(a, b) + 1).map((c) => c.id),
        ),
      );
    } else if (e.metaKey || e.ctrlKey) {
      setSelected((old) => {
        const next = new Set(old);
        next.has(card.id) ? next.delete(card.id) : next.add(card.id);
        return next;
      });
    } else setSelected(new Set([card.id]));
    selectedAnchor.current = card.id;
    setPdfTarget(card.parentID || card.id);
  }
  function revealPDFTarget(target: SourceTarget) {
    if (!project) return;
    setPdfTarget(target.id);
    const card = project.cards.find((c) => c.id === target.id);
    if (mode === "source" || !card) {
      switchMode("source");
      setNavigate({
        path: target.file,
        line: target.line,
        nonce: Date.now(),
        startUTF16: target.startUTF16,
        endUTF16: target.endUTF16,
      });
    } else {
      setSelected(new Set([card.id]));
      setFilter("all");
      setKind("all");
      setSearch("");
      const id = project.id;
      requestAnimationFrame(() => {
        if (projectRef.current?.id === id)
          grid.current
            ?.querySelector(`[data-id="${card.id}"]`)
            ?.scrollIntoView({ block: "center" });
      });
    }
  }
  const selectedCard = project?.cards.find((c) => selected.has(c.id));
  useEffect(() => {
    let stale = false;
    if (pdf?.format === "latex" && pdfTarget && project) {
      const target =
        pdf.targets?.find((c) => c.id === pdfTarget) ||
        project.cards.find((c) => c.id === pdfTarget) ||
        project.outline?.find((h) => h.id === pdfTarget);
      if (target?.file)
        void rpc<{ page: number; x: number; y: number }>("pdf_navigate", {
          key: "pdf-forward",
          document: pdf,
          file: target.file,
          line: target.line,
        })
          .then((anchor) => {
            if (!stale)
              setPdf((current) =>
                current?.pdfPath === pdf.pdfPath
                  ? {
                      ...current,
                      anchors: [
                        ...current.anchors.filter(
                          (a) => a.targetID !== pdfTarget,
                        ),
                        { ...anchor, targetID: pdfTarget },
                      ],
                    }
                  : current,
              );
          })
          .catch(() => {});
    }
    return () => {
      stale = true;
      void rpc("cancel", { key: "pdf-forward" });
    };
  }, [pdf?.pdfPath, pdfTarget]);
  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (document.documentElement.inert) return;
      const target = event.target as HTMLElement;
      if (event.isComposing || target.closest(".key-recorder")) return;
      const command = commandForEvent(event, keys);
      if (
        command &&
        ["zoomWindowIn", "zoomWindowOut", "resetWindowZoom"].includes(command)
      ) {
        event.preventDefault();
        event.stopPropagation();
        setWindowZoom((zoom) =>
          command === "resetWindowZoom"
            ? 1
            : Math.max(
                0.5,
                Math.min(
                  2,
                  Math.round(
                    (zoom + (command === "zoomWindowIn" ? 0.1 : -0.1)) * 10,
                  ) / 10,
                ),
              ),
        );
        return;
      }
      if (!ready || setupRequired) return;
      if (
        event.metaKey &&
        !event.shiftKey &&
        event.key.toLowerCase() === "w" &&
        project
      ) {
        event.preventDefault();
        event.stopPropagation();
        if (
          mode === "source" &&
          target.closest(".source-container") &&
          target.closest(".monaco-editor")
        )
          window.dispatchEvent(
            new CustomEvent("source-command", { detail: "closeSourceTab" }),
          );
        else void closeWorkspace(project.id).catch(report);
        return;
      }
      const inModal = !!target.closest('[role="dialog"]');
      if (inModal) return;
      const typing = !!target.closest(
        'input,textarea,.monaco-editor,[contenteditable="true"]',
      );
      if (
        event.metaKey &&
        event.key === "a" &&
        !typing &&
        mode === "library" &&
        project
      ) {
        event.preventDefault();
        setSelected(new Set(visible.map((c) => c.id)));
        return;
      }
      if (
        (event.metaKey || event.ctrlKey) &&
        !event.shiftKey &&
        !event.altKey &&
        event.key.toLowerCase() === "c" &&
        !typing &&
        mode === "library" &&
        project &&
        selected.size > 0
      ) {
        event.preventDefault();
        event.stopPropagation();
        void copyImages();
        return;
      }
      if (event.metaKey && event.key === ",") {
        event.preventDefault();
        setShowSettings(true);
        return;
      }
      if (!command || (!project && command === "duplicate")) return;
      if (
        typing &&
        [
          "find",
          "duplicate",
          "edit",
          "newEquation",
          "newFigure",
          "saveEquation",
        ].includes(command)
      )
        return;
      event.preventDefault();
      event.stopPropagation();
      void execute(command).catch(report);
    };
    window.addEventListener("keydown", handler, true);
    return () => window.removeEventListener("keydown", handler, true);
  }, [
    keys,
    mode,
    project,
    selected,
    visible,
    drafts,
    pdfOpen,
    inspector,
    pin,
    ready,
    setupRequired,
  ]);
  async function execute(id: string) {
    if (!ready || setupRequired) return;
    if (id === "instant")
      return rpc("instant_open", { project: project?.id }).catch(report);
    if (id === "open") return openProject();
    if (id === "newProject") {
      setNewProject(true);
      return;
    }
    if (id === "projects") {
      return showProjects();
    }
    if (id === "projectSource") {
      switchMode(mode === "library" ? "source" : "library");
      return;
    }
    if (id === "pin") {
      await getCurrentWindow().setAlwaysOnTop(!pin);
      setPin(!pin);
      await updateSettings({ pinned: !pin });
      return;
    }
    if (id === "fullScreen") {
      const win = getCurrentWindow();
      await win.setFullscreen(!(await win.isFullscreen()));
      return;
    }
    if (id === "togglePDF") {
      if (pdfOpen) closePdf();
      else void buildPdf();
      return;
    }
    if (id === "rebuildPDF") return buildPdf();
    if (
      ["previousPage", "nextPage", "zoomIn", "zoomOut", "fitPDF"].includes(id)
    ) {
      window.dispatchEvent(new CustomEvent("pdf-command", { detail: id }));
      return;
    }
    if (id === "rescan") return rescan();
    if (id === "inspector") {
      setInspector(!inspector);
      return;
    }
    if (
      mode === "source" &&
      [
        "nextSourceTab",
        "previousSourceTab",
        "splitSource",
        "splitSourceDown",
        "saveEquation",
      ].includes(id)
    ) {
      window.dispatchEvent(new CustomEvent("source-command", { detail: id }));
      return;
    }
    if (id === "newEquation") editCard();
    if (id === "newFigure") editCard(undefined, true);
    if (id === "find") searchInput.current?.focus();
    if (id === "edit") editCard(selectedCard);
    if (id === "duplicate" && selectedCard)
      editCard({ ...selectedCard, id: "custom-" + uid(), origin: "custom" });
    if (id === "copySVG") await copyImages();
    if (id === "exportSVG") await exportImage();
  }
  async function label(name?: string) {
    if (!project) return;
    const value = name || (await ask("New Label"));
    if (!value) return;
    await persist({
      ...project,
      sections: [...(project.sections || []), { id: uid(), name: value }],
    });
  }
  const title = mode === "source" ? "Source" : "Library";
  const desktopBackground = useDesktopBackground();
  return (
    <div
      className="app"
      onDragOver={(event) => event.preventDefault()}
      onDropCapture={(event) => dropSVG(event, report, "workspace")}
      onDrop={(event) => event.preventDefault()}
    >
      {desktopBackground && (
        <div
          className="desktop-wallpaper"
          style={{ backgroundImage: `url(${desktopBackground})` }}
        />
      )}
      <div className="titlebar" data-tauri-drag-region>
        <span data-tauri-drag-region>Mathlidelux</span>
      </div>
      {ready && !setupRequired && (
        <WorkspaceTabs
          workspaces={workspaces}
          active={project?.id || null}
          onSelect={(id) => void selectWorkspace(id).catch(report)}
          onClose={(id) => void closeWorkspace(id).catch(report)}
        />
      )}
      {workspaces
        .filter((item) => item.project.id !== project?.id)
        .map((item) => (
          <WorkspaceArtifacts key={item.project.id} workspace={item} />
        ))}
      {ready && !setupRequired && (
        <ProjectRecovery
          problems={projectProblems}
          onProblems={setProjectProblems}
          onProjects={(next) => void reconcileProjects(next).catch(report)}
        />
      )}
      {!ready ? (
        <div className="empty">
          <LoaderCircle className="spin" />
          <span>Opening…</span>
        </div>
      ) : setupRequired ? (
        <ProjectSetup
          initialDirectory={
            settings.projectDirectory || defaultProjectDirectory
          }
          onComplete={(data) => {
            preferencesDocument.accept({
              ...data.settings,
              value: { ...initialPreferences, ...data.settings.value },
            });
            setProjects(data.projects);
            setSetupRequired(false);
          }}
        />
      ) : !project ? (
        <ProjectsPage
          projects={projects}
          onProjects={(next) => void reconcileProjects(next).catch(report)}
          settings={settings}
          onSettings={(s) => void updateSettings(s)}
          onOpen={(p) => void openRecent(p)}
          onOpenDialog={(type) => void openProject(type)}
          onNew={() => setNewProject(true)}
          onSettingsDialog={() => setShowSettings(true)}
          onInstant={() => void execute("instant")}
          onError={setError}
          onNotice={setNotice}
          defaultDirectory={
            settings.projectDirectory || defaultProjectDirectory
          }
          blocked={showSettings || newProject || !!prompt}
        />
      ) : (
        <div className="workspace">
          <aside className="sidebar">
            <button
              className="back"
              onClick={() => void execute("projects").catch(report)}
            >
              <ArrowLeft size={17} />
              Projects
            </button>
            <div className="project-title">
              <FolderOpen size={24} />
              <h2 title={project.root}>{projectName(project.root)}</h2>
            </div>
            <div className="mode-switch">
              <button
                className={mode === "library" ? "active" : ""}
                onClick={() => switchMode("library")}
              >
                Library
              </button>
              <button
                className={mode === "source" ? "active" : ""}
                onClick={() => switchMode("source")}
              >
                Source
              </button>
            </div>
            <div
              id="source-sidebar"
              style={{ display: mode === "source" ? "flex" : "none" }}
            />
            {mode === "library" && (
              <LibraryNavigation
                project={project}
                filter={filter}
                setFilter={setFilter}
                collapsed={collapsed}
                setCollapsed={setCollapsed}
                onOutline={setPdfTarget}
                onNewLabel={() => void label()}
              />
            )}
            <footer>
              <button onClick={() => setShowProjectSettings(true)}>
                <Settings2 size={15} />
                Project Settings
              </button>
            </footer>
          </aside>
          <div className="main-column">
            <header className="workspace-heading">
              <div>
                <h1>{title}</h1>
                <span className="spacer" />
                <button
                  title="Open with Instant Mode"
                  aria-label="Open with Instant Mode"
                  onClick={() => void execute("instant")}
                >
                  <Zap size={17} />
                </button>
                <button
                  title="Always on top"
                  className={pin ? "active" : ""}
                  onClick={() => void execute("pin")}
                >
                  <Pin size={17} />
                </button>
                <button
                  title="Appearance"
                  onClick={() =>
                    void updateSettings({
                      appearance: dark ? "light" : "dark",
                    })
                  }
                >
                  <SunMoon size={17} />
                </button>
                <button
                  title="Toggle PDF"
                  className={pdfOpen ? "active" : ""}
                  onClick={() => void execute("togglePDF")}
                >
                  <FileText size={17} />
                </button>
                {mode === "library" && (
                  <>
                    <button
                      title="Inspector"
                      className={inspector ? "active" : ""}
                      onClick={() => setInspector(!inspector)}
                    >
                      <PanelLeftClose size={17} />
                    </button>
                    <button title="New equation" onClick={() => editCard()}>
                      <Plus size={18} />
                    </button>
                    <button
                      title="Rescan"
                      disabled={!!busy}
                      onClick={() => void rescan()}
                    >
                      <RefreshCw size={16} className={busy ? "spin" : ""} />
                    </button>
                  </>
                )}
                <button title="Settings" onClick={() => setShowSettings(true)}>
                  <Settings2 size={17} />
                </button>
              </div>
              {mode === "library" && (
                <div className="library-controls">
                  <div className="segmented">
                    {[
                      ["all", "All"],
                      ["inline", "Inline"],
                      ["block", "Block"],
                      ["figure", "Figures"],
                    ].map(([value, name]) => (
                      <button
                        key={value}
                        className={kind === value ? "active" : ""}
                        onClick={() => setKind(value)}
                      >
                        {name}
                      </button>
                    ))}
                  </div>
                  <span className="spacer" />
                  <div className="search">
                    <Search size={15} />
                    <input
                      ref={searchInput}
                      aria-label="Search equations"
                      placeholder="Search equations"
                      value={search}
                      onChange={(e) => setSearch(e.target.value)}
                    />
                  </div>
                </div>
              )}
            </header>
            <div className="content-row">
              <div className="central-workspace">
                <div
                  className="library"
                  ref={canvasScroll}
                  style={{ display: mode === "library" ? "flex" : "none" }}
                >
                  <div
                    className="cards-grid"
                    ref={grid}
                    style={{ zoom: canvasZoom }}
                    tabIndex={-1}
                    onPointerDown={(e) => {
                      e.currentTarget.focus({ preventScroll: true });
                      if ((e.target as HTMLElement).closest(".equation-card"))
                        return;
                      const origin = { x: e.clientX, y: e.clientY },
                        previous = e.metaKey
                          ? new Set(selected)
                          : new Set<string>();
                      setSelected(previous);
                      const el = e.currentTarget;
                      el.setPointerCapture(e.pointerId);
                      const move = (ev: PointerEvent) => {
                        const box = {
                          x: Math.min(origin.x, ev.clientX),
                          y: Math.min(origin.y, ev.clientY),
                          width: Math.abs(origin.x - ev.clientX),
                          height: Math.abs(origin.y - ev.clientY),
                        };
                        setMarquee(box);
                        const next = new Set(previous);
                        el.querySelectorAll<HTMLElement>(
                          ".equation-card",
                        ).forEach((card) => {
                          const r = card.getBoundingClientRect();
                          if (
                            r.left < box.x + box.width &&
                            r.right > box.x &&
                            r.top < box.y + box.height &&
                            r.bottom > box.y
                          )
                            next.add(card.dataset.id!);
                        });
                        setSelected(next);
                      };
                      el.addEventListener("pointermove", move);
                      el.addEventListener(
                        "pointerup",
                        () => {
                          el.removeEventListener("pointermove", move);
                          setMarquee(null);
                        },
                        { once: true },
                      );
                    }}
                  >
                    {visible.map((card) => (
                      <article
                        tabIndex={0}
                        role="button"
                        aria-label={card.name}
                        aria-selected={selected.has(card.id)}
                        data-id={card.id}
                        className={`equation-card ${selected.has(card.id) ? "selected" : ""}`}
                        key={card.id}
                        onClick={(e) => choose(card, e)}
                        onDoubleClick={() => editCard(card)}
                        onContextMenu={(e) => {
                          e.preventDefault();
                          if (!selected.has(card.id))
                            setSelected(new Set([card.id]));
                          setContext({ x: e.clientX, y: e.clientY, card });
                        }}
                      >
                        <small>
                          {card.origin === "custom"
                            ? "Variant"
                            : card.kind === "figure"
                              ? "Figure"
                              : card.kind === "inline"
                                ? "Inline"
                                : "Block"}
                        </small>
                        <ImagePreview
                          card={card}
                          onDrag={(c) => void drag(c)}
                        />
                        <strong>{card.name}</strong>
                        <span className="card-path">
                          {card.file
                            ? card.line
                              ? `${card.file}:${card.line}`
                              : card.file
                            : "Library"}
                        </span>
                      </article>
                    ))}
                    {!visible.length && (
                      <div className="empty">
                        <Grid2X2 size={30} />
                        <span>{busy || "No equations"}</span>
                      </div>
                    )}
                  </div>
                  <footer className="library-footer">
                    <span>
                      {selected.size > 1
                        ? `${selected.size} selected`
                        : `${visible.length} items`}
                    </span>
                    {busy && (
                      <>
                        <LoaderCircle size={13} className="spin" />
                        <span>{busy}</span>
                      </>
                    )}
                    <span className="spacer" />
                    <button
                      className="zoom-level"
                      title="Reset canvas zoom (Ctrl + scroll to zoom)"
                      aria-label={`Canvas zoom ${Math.round(canvasZoom * 100)}%. Reset to 100%`}
                      onClick={() => setCanvasZoom(1)}
                    >
                      {Math.round(canvasZoom * 100)}%
                    </button>
                    <select
                      title="Export format"
                      value={project.exportFormat || "svg"}
                      onChange={(e) =>
                        void persist({
                          ...project,
                          exportFormat: e.target.value,
                        })
                      }
                    >
                      {["svg", "png", "jpeg", "pdf"].map((f) => (
                        <option value={f} key={f}>
                          {f.toUpperCase()}
                        </option>
                      ))}
                    </select>
                    <select
                      title="Default output font size"
                      value={project.exportFontSize || 0}
                      onChange={(e) =>
                        void persist({
                          ...project,
                          exportFontSize: +e.target.value,
                        })
                      }
                    >
                      <OutputSizeOptions
                        value={project.exportFontSize || 0}
                        sourceSizes={project.cards.map(
                          (card) => card.sourceFontSize,
                        )}
                      />
                    </select>
                    <button
                      title={
                        selected.size > 1
                          ? `Copy ${selected.size} images (⌘C)`
                          : "Copy image (⌘C)"
                      }
                      disabled={!selectedCard}
                      onClick={() => void copyImages()}
                    >
                      <ClipboardCopy size={16} />
                    </button>
                    <button
                      title="Export image"
                      disabled={!selectedCard}
                      onClick={() => void exportImage()}
                    >
                      <Download size={16} />
                    </button>
                  </footer>
                </div>
                {sourceMounted && (
                  <div
                    className="source-container"
                    style={{ display: mode === "source" ? "flex" : "none" }}
                  >
                    <SourceWorkspace
                      key={project.id}
                      project={project}
                      autoSave={settings.autoSave}
                      session={sourceSessions.current.get(project.id)}
                      onSession={(session) => {
                        if (
                          workspacesRef.current.some(
                            (item) => item.project.id === project.id,
                          )
                        )
                          sourceSessions.current.set(project.id, session);
                      }}
                      onDrafts={setDrafts}
                      onError={setError}
                      ask={ask}
                      navigate={navigate}
                      onCursor={(file, offset) => {
                        if (mode !== "source" || !pdf) return;
                        const target = targetAtSource(
                          pdf.targets || [],
                          file,
                          offset,
                        );
                        if (target) setPdfTarget(target.id);
                      }}
                      beforeMutation={async () => {
                        await cancelJobs();
                        await writer.current.flush();
                      }}
                      onProjectChanged={(patch) => {
                        if (!projectRef.current) return;
                        const next = { ...projectRef.current, ...patch };
                        projectRef.current = next;
                        setProject(next);
                        setProjects((all) =>
                          all.map((item) =>
                            item.id === next.id ? next : item,
                          ),
                        );
                      }}
                      onSaved={() => {
                        setPdf(null);
                        setLibraryStale(true);
                      }}
                    />
                  </div>
                )}
              </div>
              {inspector && mode === "library" && selectedCard && (
                <aside className="inspector">
                  <header>
                    <strong>Source</strong>
                    <button
                      title="Close inspector"
                      onClick={() => setInspector(false)}
                    >
                      <X size={16} />
                    </button>
                  </header>
                  <ImagePreview
                    card={selectedCard}
                    onDrag={(c) => void drag(c)}
                  />
                  <pre>{selectedCard.tex}</pre>
                  <label>
                    Output size
                    <select
                      value={selectedCard.exportFontSize ?? -1}
                      onChange={(e) =>
                        void persist({
                          ...project,
                          cards: project.cards.map((c) =>
                            c.id === selectedCard.id
                              ? {
                                  ...c,
                                  exportFontSize:
                                    +e.target.value < 0
                                      ? undefined
                                      : +e.target.value,
                                }
                              : c,
                          ),
                        })
                      }
                    >
                      <OutputSizeOptions
                        value={selectedCard.exportFontSize ?? -1}
                        sourceSizes={[selectedCard.sourceFontSize]}
                        projectSize={project.exportFontSize || 0}
                      />
                    </select>
                  </label>
                  <button
                    className="soft"
                    onClick={() => editCard(selectedCard)}
                  >
                    Edit Variant
                  </button>
                  {selectedCard.file && (
                    <button onClick={() => showSource(selectedCard)}>
                      <Code2 size={16} />
                      Open Source
                    </button>
                  )}
                  {selectedCard.error && (
                    <pre className="error-box">{selectedCard.error}</pre>
                  )}
                </aside>
              )}
            </div>
          </div>
          {pdfOpen && (
            <>
              <div
                className="pdf-resizer"
                onPointerDown={(e) => {
                  const el = e.currentTarget;
                  el.setPointerCapture(e.pointerId);
                  let last = settings.pdfWidth;
                  const move = (ev: PointerEvent) => {
                    last = Math.min(
                      innerWidth * 0.65,
                      Math.max(280, innerWidth - ev.clientX),
                    );
                    setSettings((old) => ({ ...old, pdfWidth: last }));
                  };
                  el.addEventListener("pointermove", move);
                  el.addEventListener(
                    "pointerup",
                    () => {
                      el.removeEventListener("pointermove", move);
                      void updateSettings({ pdfWidth: last });
                    },
                    { once: true },
                  );
                }}
              />
              <div className="pdf-column" style={{ width: settings.pdfWidth }}>
                <PdfPane
                  key={project.id}
                  document={pdf}
                  project={project}
                  building={pdfBusy}
                  onBuild={() => void buildPdf()}
                  onClose={closePdf}
                  initialView={pdfViews.current.get(project.id)}
                  onView={(view) => pdfViews.current.set(project.id, view)}
                  target={pdfTarget}
                  onSource={(file, line) => {
                    const matches = targetsAtLine(
                      pdf?.targets || [],
                      file,
                      line,
                    );
                    if (matches.length === 1) revealPDFTarget(matches[0]);
                    else if (matches.length > 1 && mode === "library") {
                      // Some TeX engines return a source line without a column.
                      // Preserve that ambiguity instead of choosing the wrong equation.
                      revealPDFTarget(matches[0]);
                      setSelected(new Set(matches.map((t) => t.id)));
                    } else {
                      switchMode("source");
                      setNavigate({ path: file, line, nonce: Date.now() });
                    }
                  }}
                  onNavigate={(anchor) => {
                    const target = pdf?.targets?.find(
                      (t) => t.id === anchor.targetID,
                    );
                    if (target) {
                      revealPDFTarget(target);
                      return;
                    }
                    const card = project.cards.find(
                      (c) => c.id === anchor.targetID,
                    );
                    if (card) {
                      setSelected(new Set([card.id]));
                      setFilter("all");
                      setKind("all");
                      setSearch("");
                      if (mode === "source") showSource(card);
                      else
                        setTimeout(
                          () =>
                            grid.current
                              ?.querySelector(`[data-id="${card.id}"]`)
                              ?.scrollIntoView({ block: "center" }),
                          50,
                        );
                    } else {
                      const h = project.outline.find(
                        (h) => h.id === anchor.targetID,
                      );
                      if (h) {
                        if (mode === "source")
                          setNavigate({
                            path: h.file,
                            line: h.line,
                            nonce: Date.now(),
                          });
                        else setFilter(h.id);
                      }
                    }
                  }}
                />
              </div>
            </>
          )}
        </div>
      )}
      {error && (
        <div role="alert" className="toast error-toast">
          <TriangleAlert size={18} />
          <div>{error}</div>
          <button aria-label="Dismiss error" onClick={() => setError("")}>
            <X size={17} />
          </button>
        </div>
      )}
      {notice && <div className="toast">{notice}</div>}
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
      {showSettings && (
        <Settings
          settings={settings}
          defaultProjectDirectory={defaultProjectDirectory}
          themes={themes}
          theme={theme}
          keys={keys}
          onSettings={(s) => void updateSettings(s)}
          onTheme={(t) => void updateTheme(t)}
          onKeys={(k) => void updateKeys(k)}
          onReload={(file) => {
            if (file === "preferences.json")
              return preferencesDocument.reload(async () => {
                const snapshot = await rpc<SettingsSnapshot<Preferences>>(
                  "reload_settings",
                  { file },
                );
                return {
                  ...snapshot,
                  value: { ...initialPreferences, ...snapshot.value },
                };
              });
            if (file === "theme.json")
              return themeDocument.reload(() =>
                rpc("reload_settings", { file }),
              );
            return keysDocument.reload(() => rpc("reload_settings", { file }));
          }}
          onClose={() => setShowSettings(false)}
          onError={setError}
        />
      )}
      {showProjectSettings && project && (
        <Modal
          title="Project Settings"
          onClose={() => setShowProjectSettings(false)}
        >
          <div className="settings-content">
            <label className="setting-row">
              Main file
              <select
                value={project.main}
                onChange={(e) => {
                  void persist({ ...project, main: e.target.value }).then(() =>
                    rescan(),
                  );
                }}
              >
                {project.candidates
                  ?.filter((c) =>
                    c.endsWith(project.format === "typst" ? ".typ" : ".tex"),
                  )
                  .map((c) => (
                    <option key={c}>{c}</option>
                  ))}
              </select>
            </label>
            {project.format === "latex" && (
              <label className="setting-row">
                Compiler
                <select
                  value={project.engine}
                  onChange={(e) =>
                    void persist({ ...project, engine: e.target.value })
                  }
                >
                  {["project", "pdflatex", "xelatex", "lualatex"].map((c) => (
                    <option key={c}>{c}</option>
                  ))}
                </select>
              </label>
            )}
            <button onClick={() => void rpc("reveal", { project: project.id })}>
              <FolderOpen size={16} />
              Reveal Project in Finder
            </button>
            {!!project.warnings?.length && (
              <details>
                <summary>{project.warnings.length} scan notices</summary>
                <pre>{project.warnings.join("\n")}</pre>
              </details>
            )}
          </div>
        </Modal>
      )}
      {editing && project && (
        <VariantEditor
          project={project}
          editing={editing}
          setEditing={setEditing}
          editPreview={editPreview}
          editBusy={editBusy}
          onClose={closeEditor}
          onSave={() => void saveVariant().catch(report)}
          onRender={() => setEditing({ ...editing, requiresRender: false })}
          onDrag={(card) =>
            void rpc<string>("preview_export", {
              project: project.id,
              card,
              format: project.exportFormat || "svg",
              fontSize: editing.exportFontSize ?? project.exportFontSize,
            })
              .then((path) =>
                startDrag({
                  item: [path],
                  icon: card.pngPath!,
                  mode: "copy",
                }),
              )
              .catch(report)
          }
        />
      )}
      {newProject && (
        <NewProjectDialog
          configuration={templates}
          defaultDirectory={
            settings.projectDirectory || defaultProjectDirectory
          }
          onConfiguration={setTemplates}
          onCreated={(p) => {
            setNewProject(false);
            void activate(p);
          }}
          onClose={() => setNewProject(false)}
          onError={setError}
          ask={ask}
        />
      )}
      {prompt && (
        <Prompt
          key={prompt.title}
          {...prompt}
          onDone={(value) => {
            prompt.resolve(value);
            setPrompt(null);
          }}
        />
      )}
      {context && (
        <ContextMenu
          x={context.x}
          y={context.y}
          onClose={() => setContext(null)}
          items={[
            {
              label: "Edit Variant",
              action: () => editCard(context.card),
            },
            {
              label: "Open Source",
              disabled: !context.card.file,
              action: () => showSource(context.card!),
            },
            { label: "Copy Image", action: () => void copyImages() },
            {
              label: "Export Image…",
              action: () => void exportImage(),
            },
            ...(project?.sections?.map((section) => ({
              label: "Label: " + section.name,
              action: () =>
                void persist({
                  ...project!,
                  sectionAssignments: {
                    ...project!.sectionAssignments,
                    ...Object.fromEntries(
                      [...selected].map((id) => [id, section.id]),
                    ),
                  },
                }),
            })) || []),
            ...(context.card.origin === "custom"
              ? [
                  {
                    label: "Delete Variant",
                    action: () =>
                      void persist({
                        ...project!,
                        cards: project!.cards.filter(
                          (c) => c.id !== context.card!.id,
                        ),
                      }),
                  },
                ]
              : []),
          ]}
        />
      )}
    </div>
  );
}
