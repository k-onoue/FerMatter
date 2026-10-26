// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import type { Card, Project } from "./types";
import { defaults } from "./appearance";
import App from "./App";
import { flushWindow } from "./persistence";
import { disposeProjectModels } from "./editor";
import { initialInstant } from "./instant";
import { isWindows } from "./platform";
const { rpc, save, events } = vi.hoisted(() => ({
  rpc: vi.fn(),
  save: vi.fn(),
  events: new Map<string, Set<() => void>>(),
}));
vi.mock("./api", () => ({
  rpc,
  uid: () => crypto.randomUUID(),
  projectName: () => "Synthetic",
  asset: async () => "",
}));
vi.mock("./editor", () => ({
  setProjectCompletions: vi.fn(),
  disposeProjectModels: vi.fn(),
  CodeEditor: () => null,
}));
vi.mock("./useAppearance", () => ({ useAppearance: () => {} }));
vi.mock("./useDesktopBackground", () => ({ useDesktopBackground: () => null }));
vi.mock("./PdfPane", () => ({ PdfPane: () => null }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ save, open: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: async (name: string, callback: () => void) => {
    if (!events.has(name)) events.set(name, new Set());
    events.get(name)!.add(callback);
    return () => events.get(name)?.delete(callback);
  },
}));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ setAlwaysOnTop: async () => {} }),
}));
vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({ setZoom: async () => {} }),
}));
vi.mock("@crabnebula/tauri-plugin-drag", () => ({ startDrag: vi.fn() }));
vi.mock("./ProjectsPage", () => ({
  ProjectsPage: ({
    projects,
    onOpen,
    onProjects,
  }: {
    projects: Project[];
    onOpen: (p: Project) => void;
    onProjects: (p: Project[]) => void;
  }) => (
    <>
      {projects.map((p) => (
        <button key={p.id} onClick={() => onOpen(p)}>
          Open {p.id === "p" ? "synthetic" : p.id}
        </button>
      ))}
      <button onClick={() => onProjects(projects.slice(1))}>
        Remove first project
      </button>
    </>
  ),
}));
vi.mock("./VariantEditor", () => ({
  VariantEditor: ({
    editing,
    setEditing,
    onSave,
    onRender,
  }: {
    editing: Card;
    setEditing: (c: Card) => void;
    onSave: () => void;
    onRender: () => void;
  }) => (
    <section>
      <textarea
        aria-label="Variant source"
        value={editing.tex}
        onChange={(e) => setEditing({ ...editing, tex: e.target.value })}
      />
      <button onClick={onSave}>Save test variant</button>
      <button
        onClick={() => setEditing({ ...editing, instantColor: "2563eb" })}
      >
        Set test color
      </button>
      <button
        onClick={() => setEditing({ ...editing, instantColor: undefined })}
      >
        Reset test color
      </button>
      {editing.requiresRender === true && (
        <button onClick={onRender}>Render imported source</button>
      )}
    </section>
  ),
}));
let project: Project;
let saved: Project[];
let failSave: boolean;
let delaySave: ((value: Project) => Promise<void>) | undefined;
let scanImpl: (() => Promise<Project>) | undefined;
let availableProjects: Project[];
let pendingImports: {
  token: string;
  project: Project;
  draft: ReturnType<typeof initialInstant>;
}[];
beforeEach(() => {
  vi.stubGlobal("matchMedia", () => ({
    matches: false,
    addEventListener() {},
    removeEventListener() {},
  }));
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      disconnect() {}
    },
  );
  project = {
    id: "p",
    extractionRevision: "extraction",
    root: "/synthetic",
    main: "main.typ",
    format: "typst",
    engine: "typst",
    cards: [
      {
        id: "source",
        name: "Source",
        tex: "x",
        kind: "inline",
        origin: "source",
        sourceOrder: 0,
        status: "pending",
      },
    ],
    outline: [],
    sections: [],
    sectionAssignments: {},
    customOrder: [],
    exportFontSize: 0,
    exportFormat: "svg",
    candidates: ["main.typ"],
  } as Project;
  saved = [];
  availableProjects = [project];
  pendingImports = [];
  vi.mocked(disposeProjectModels).mockClear();
  save.mockReset().mockResolvedValue("/synthetic/out.svg");
  failSave = false;
  delaySave = undefined;
  scanImpl = undefined;
  rpc
    .mockReset()
    .mockImplementation(
      async (operation: string, payload: Record<string, unknown>) => {
        if (operation === "startup")
          return {
            projects: availableProjects,
            projectProblems: [],
            setupRequired: false,
            defaultProjectDirectory: "/synthetic",
            settings: {
              value: { appearance: "light", autoSave: false, opacity: 1 },
              revision: "settings",
            },
            themes: [defaults],
            theme: { value: defaults, revision: "theme" },
            keybindings: {
              value: {
                schemaVersion: 1,
                kind: "mathlidelux.keybindings",
                bindings: {},
              },
              revision: "keys",
            },
            extraction: {
              value: { ignoreMathInTables: false },
              revision: "extraction",
            },
            templates: { custom: [] },
          };
        if (operation === "incoming_import") {
          if (payload?.token) {
            expect(pendingImports[0]?.token).toBe(payload.token);
            pendingImports.shift();
          }
          return pendingImports[0] || null;
        }
        if (operation === "open_project")
          return (
            availableProjects.find((p) => p.root === payload.path) || project
          );
        if (operation === "tree") return [];
        if (operation === "read_drafts") return { buffers: {}, warnings: [] };
        if (operation === "read_file")
          return {
            path: payload.path,
            text: "$ x $",
            revision: "source",
            bom: false,
            newline: "\n",
          };
        if (operation === "save_project") {
          if (failSave) throw Error("disk full");
          const next = structuredClone(payload.value as Project);
          await delaySave?.(next);
          saved.push(next);
          project = next;
          availableProjects = availableProjects.map((p) =>
            p.id === next.id ? next : p,
          );
          return null;
        }
        if (operation === "scan") return scanImpl ? scanImpl() : project;
        if (operation === "render") return [];
        return null;
      },
    );
});

it("reuses project tabs and restores each Library/Source mode and search without disposing models", async () => {
  const other = { ...project, id: "other", root: "/other" };
  availableProjects.push(other);
  await openProject();
  fireEvent.change(screen.getByPlaceholderText("Search equations"), {
    target: { value: "remember filter" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Source" }));
  fireEvent.click(screen.getByRole("tab", { name: "Projects" }));
  fireEvent.click(await screen.findByRole("button", { name: "Open other" }));
  await screen.findByTitle("New equation");
  expect(
    (screen.getByPlaceholderText("Search equations") as HTMLInputElement).value,
  ).toBe("");
  fireEvent.click(screen.getByTitle("/synthetic"));
  await waitFor(() =>
    expect(
      screen
        .getByRole("button", { name: "Source" })
        .classList.contains("active"),
    ).toBe(true),
  );
  fireEvent.click(screen.getByRole("button", { name: "Library" }));
  expect(
    (screen.getByPlaceholderText("Search equations") as HTMLInputElement).value,
  ).toBe("remember filter");
  expect(screen.getAllByRole("tab")).toHaveLength(3);
  expect(disposeProjectModels).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("tab", { name: "Projects" }));
  fireEvent.click(
    await screen.findByRole("button", { name: "Open synthetic" }),
  );
  await screen.findByTitle("New equation");
  expect(screen.getAllByRole("tab")).toHaveLength(3);
});

it("closing a project tab saves its draft and disposes only that project's models", async () => {
  await openProject();
  edit("draft before tab close");
  failSave = true;
  fireEvent.keyDown(document.body, {
    key: "w",
    metaKey: !isWindows(),
    ctrlKey: isWindows(),
  });
  await screen.findByText(/disk full/);
  expect(screen.getAllByRole("tab")).toHaveLength(2);
  expect(disposeProjectModels).not.toHaveBeenCalled();
  failSave = false;
  fireEvent.keyDown(document.body, {
    key: "w",
    metaKey: !isWindows(),
    ctrlKey: isWindows(),
  });
  await screen.findByRole("button", { name: "Open synthetic" });
  expect((saved.at(-1)?.libraryDraft as Card).tex).toBe(
    "draft before tab close",
  );
  expect(screen.getAllByRole("tab")).toHaveLength(1);
  expect(disposeProjectModels).toHaveBeenCalledExactlyOnceWith("p");
});

it("reopening an existing project with another entrypoint persists it before scanning and preserves its draft", async () => {
  await openProject();
  edit("retained on entrypoint change");
  fireEvent.click(screen.getByRole("tab", { name: "Projects" }));
  await screen.findByRole("button", { name: "Open synthetic" });
  const base = rpc.getMockImplementation()!;
  rpc.mockImplementation((operation, ...args) => {
    if (operation === "open_project") {
      project = {
        ...project,
        main: "other.typ",
        candidates: ["main.typ", "other.typ"],
      };
      return Promise.resolve(project);
    }
    return base(operation, ...args);
  });
  const scannedEntrypoints: string[] = [];
  scanImpl = async () => {
    scannedEntrypoints.push(project.main);
    return structuredClone(project);
  };
  fireEvent.click(screen.getByRole("button", { name: "Open synthetic" }));
  await waitFor(() => expect(scannedEntrypoints).toEqual(["other.typ"]));
  expect(saved.at(-1)?.main).toBe("other.typ");
  expect((saved.at(-1)?.libraryDraft as Card).tex).toBe(
    "retained on entrypoint change",
  );
  expect(screen.getAllByRole("tab")).toHaveLength(2);
});

it("SVG restoration reuses an existing tab, preserves its prior draft, and previews automatically", async () => {
  await openProject();
  edit("my existing scratch");
  const draft = { ...initialInstant(), source: "imported source" };
  pendingImports.push({ token: "svg-one", project, draft });
  await act(async () => {
    for (const event of events.get("workspace-import-pending") || []) event();
  });
  await waitFor(() =>
    expect(
      (
        screen.getByRole("textbox", {
          name: "Variant source",
        }) as HTMLTextAreaElement
      ).value,
    ).toBe("imported source"),
  );
  await waitFor(() => expect(pendingImports).toHaveLength(0));
  expect(screen.getAllByRole("tab")).toHaveLength(2);
  expect(
    saved.at(-1)?.cards.some((card) => card.tex === "my existing scratch"),
  ).toBe(true);
  expect((saved.at(-1)?.libraryDraft as Card).requiresRender).toBe(false);
  await waitFor(() =>
    expect(
      rpc.mock.calls.some(
        ([op, payload]) =>
          op === "render" &&
          payload.cards?.some((card: Card) => card.tex === "imported source"),
      ),
    ).toBe(true),
  );
  expect(rpc.mock.calls.some(([op]) => op === "save_file")).toBe(false);
});

it("opening a legacy imported Library draft enables preview without changing original cards or source files", async () => {
  const original = structuredClone(project.cards);
  project.libraryDraft = {
    ...project.cards[0],
    id: "restored",
    origin: "custom",
    tex: "legacy import",
    requiresRender: true,
  };
  await openProject();
  await waitFor(() =>
    expect(
      rpc.mock.calls.some(
        ([op, payload]) =>
          op === "render" &&
          payload.cards?.some(
            (card: Card) =>
              card.tex === "legacy import" && card.requiresRender === false,
          ),
      ),
    ).toBe(true),
  );
  await act(async () => flushWindow());
  expect(saved.at(-1)?.cards).toEqual(original);
  expect((saved.at(-1)?.libraryDraft as Card).requiresRender).toBe(false);
  expect(rpc.mock.calls.some(([op]) => op === "save_file")).toBe(false);
});

it("native close drains a queued SVG into a new project tab before acknowledging persistence", async () => {
  await openProject();
  const other = { ...project, id: "other", root: "/other" };
  pendingImports.push({
    token: "svg-two",
    project: other,
    draft: { ...initialInstant(), source: "second project" },
  });
  await act(async () => {
    await flushWindow();
  });
  expect(pendingImports).toHaveLength(0);
  expect((saved.at(-1)?.libraryDraft as Card).tex).toBe("second project");
  expect(saved.at(-1)?.id).toBe("other");
  expect(screen.getAllByRole("tab")).toHaveLength(3);
});

it("failed SVG persistence keeps its queue token and retries without duplicating the prior draft", async () => {
  await openProject();
  edit("retained scratch");
  pendingImports.push({
    token: "retry",
    project,
    draft: { ...initialInstant(), source: "incoming" },
  });
  failSave = true;
  await act(async () => {
    await expect(flushWindow()).rejects.toThrow("disk full");
  });
  expect(pendingImports).toHaveLength(1);
  failSave = false;
  await act(async () => {
    await flushWindow();
  });
  expect(pendingImports).toHaveLength(0);
  expect(
    saved.at(-1)?.cards.filter((card) => card.tex === "retained scratch"),
  ).toHaveLength(1);
});

it("prunes tabs and their models when a project is removed from Projects", async () => {
  await openProject();
  fireEvent.click(screen.getByRole("tab", { name: "Projects" }));
  fireEvent.click(
    await screen.findByRole("button", { name: "Remove first project" }),
  );
  await waitFor(() => expect(screen.getAllByRole("tab")).toHaveLength(1));
  expect(disposeProjectModels).toHaveBeenCalledExactlyOnceWith("p");
});
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});
async function openProject() {
  render(<App />);
  fireEvent.click(
    await screen.findByRole("button", { name: "Open synthetic" }),
  );
  await screen.findByTitle("New equation");
  await waitFor(() => expect(document.documentElement.inert).toBe(false));
}
function edit(text: string) {
  fireEvent.click(screen.getByTitle("New equation"));
  fireEvent.change(screen.getByRole("textbox", { name: "Variant source" }), {
    target: { value: text },
  });
}
it("renders color-only changes and resets while preserving the source card", async () => {
  const original = structuredClone(project.cards);
  await openProject();
  edit("x");
  const editRenders = () =>
    rpc.mock.calls.filter(
      ([op, payload]) => op === "render" && payload.key === "p:edit",
    );
  await waitFor(() => expect(editRenders()).toHaveLength(1));
  fireEvent.click(screen.getByRole("button", { name: "Set test color" }));
  await waitFor(() => expect(editRenders()).toHaveLength(2));
  expect(editRenders()[1][1].cards[0].instantColor).toBe("2563eb");
  fireEvent.click(screen.getByRole("button", { name: "Reset test color" }));
  await waitFor(() => expect(editRenders()).toHaveLength(3));
  expect(editRenders()[2][1].cards[0].instantColor).toBeUndefined();
  fireEvent.click(screen.getByRole("button", { name: "Set test color" }));
  fireEvent.click(screen.getByRole("button", { name: "Save test variant" }));
  await waitFor(() =>
    expect(
      saved.some((p) =>
        p.cards.some(
          (card) => card.origin === "custom" && card.instantColor === "2563eb",
        ),
      ),
    ).toBe(true),
  );
  expect(saved.at(-1)?.cards.find((card) => card.id === "source")).toEqual(
    original[0],
  );
  expect(rpc.mock.calls.some(([op]) => op === "save_file")).toBe(false);
});
it("failed Library saving keeps the editor and blocks project switching until recovery succeeds", async () => {
  await openProject();
  edit("latest variant");
  failSave = true;
  fireEvent.click(screen.getByRole("button", { name: "Save test variant" }));
  await screen.findByText("Error: disk full");
  expect(screen.getByRole("textbox", { name: "Variant source" })).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Projects" }));
  await waitFor(() =>
    expect(
      rpc.mock.calls.filter(([op]) => op === "save_project").length,
    ).toBeGreaterThan(1),
  );
  expect(screen.queryByRole("button", { name: "Open synthetic" })).toBeNull();
  failSave = false;
  fireEvent.click(screen.getByRole("button", { name: "Projects" }));
  await screen.findByRole("button", { name: "Open synthetic" });
  expect((saved.at(-1)?.libraryDraft as Card).tex).toBe("latest variant");
  fireEvent.click(screen.getByRole("button", { name: "Open synthetic" }));
  expect(
    (
      (await screen.findByRole("textbox", {
        name: "Variant source",
      })) as HTMLTextAreaElement
    ).value,
  ).toBe("latest variant");
});
it("a late scan in the mounted app retains a variant saved while scanning", async () => {
  await openProject();
  const stale = structuredClone(project);
  let finish!: (p: Project) => void;
  scanImpl = () =>
    new Promise((resolve) => {
      finish = resolve;
    });
  fireEvent.click(screen.getByTitle("Rescan"));
  await waitFor(() => expect(finish).toBeDefined());
  edit("created during scan");
  fireEvent.click(screen.getByRole("button", { name: "Save test variant" }));
  await waitFor(() =>
    expect(
      screen.queryByRole("textbox", { name: "Variant source" }),
    ).toBeNull(),
  );
  await act(async () => {
    finish(stale);
  });
  await waitFor(() =>
    expect(rpc.mock.calls.some(([op]) => op === "render")).toBe(true),
  );
  expect(
    saved.at(-1)?.cards.some((card) => card.tex === "created during scan"),
  ).toBe(true);
});
it("typing during a Library save keeps the newer editor draft open", async () => {
  await openProject();
  edit("first");
  let finish!: () => void;
  delaySave = () =>
    new Promise((resolve) => {
      finish = resolve;
    });
  fireEvent.click(screen.getByRole("button", { name: "Save test variant" }));
  await waitFor(() => expect(finish).toBeDefined());
  fireEvent.change(screen.getByRole("textbox", { name: "Variant source" }), {
    target: { value: "newer" },
  });
  await act(async () => {
    delaySave = undefined;
    finish();
  });
  expect(
    (
      screen.getByRole("textbox", {
        name: "Variant source",
      }) as HTMLTextAreaElement
    ).value,
  ).toBe("newer");
  fireEvent.click(screen.getByRole("button", { name: "Save test variant" }));
  await waitFor(() =>
    expect(
      screen.queryByRole("textbox", { name: "Variant source" }),
    ).toBeNull(),
  );
  expect(saved.at(-1)?.cards.some((card) => card.tex === "newer")).toBe(true);
});

it("an older project-open response cannot replace the most recently requested project", async () => {
  const base = rpc.getMockImplementation()!;
  const opens: ((value: Project) => void)[] = [];
  rpc.mockImplementation((operation, ...args) =>
    operation === "open_project"
      ? new Promise((resolve) => opens.push(resolve))
      : base(operation, ...args),
  );
  render(<App />);
  const button = await screen.findByRole("button", { name: "Open synthetic" });
  fireEvent.click(button);
  fireEvent.click(button);
  await waitFor(() => expect(opens).toHaveLength(2));
  await act(async () => {
    opens[1]({
      ...project,
      id: "new",
      cards: [{ ...project.cards[0], name: "Newest project card" }],
    });
  });
  await screen.findByText("Newest project card");
  await act(async () => {
    opens[0]({
      ...project,
      id: "old",
      cards: [{ ...project.cards[0], name: "Older project card" }],
    });
  });
  expect(screen.getByText("Newest project card")).toBeTruthy();
  expect(screen.queryByText("Older project card")).toBeNull();
  expect(document.documentElement.inert).toBe(false);
});

it("closing PDF during a pending project flush never starts the obsolete compile", async () => {
  await openProject();
  edit("pending variant");
  let finish!: () => void;
  delaySave = () =>
    new Promise((resolve) => {
      finish = resolve;
    });
  fireEvent.click(screen.getByRole("button", { name: "Save test variant" }));
  await waitFor(() => expect(finish).toBeDefined());
  fireEvent.click(screen.getByTitle("Toggle PDF"));
  fireEvent.click(screen.getByTitle("Toggle PDF"));
  await act(async () => {
    delaySave = undefined;
    finish();
  });
  await waitFor(() =>
    expect(
      screen.queryByRole("textbox", { name: "Variant source" }),
    ).toBeNull(),
  );
  expect(rpc.mock.calls.filter(([op]) => op === "document")).toHaveLength(0);
});
it("reopening a closed PDF rebuilds instead of reusing a released snapshot", async () => {
  const base = rpc.getMockImplementation()!;
  rpc.mockImplementation((operation, ...args) =>
    operation === "document"
      ? Promise.resolve({
          pdfPath: "/synthetic/result.pdf",
          sourceRoot: "/synthetic/source",
          format: "typst",
          anchors: [],
          targets: [],
        })
      : base(operation, ...args),
  );
  await openProject();
  fireEvent.click(screen.getByTitle("Toggle PDF"));
  await waitFor(() =>
    expect(rpc.mock.calls.filter(([op]) => op === "document")).toHaveLength(1),
  );
  fireEvent.click(screen.getByTitle("Toggle PDF"));
  fireEvent.click(screen.getByTitle("Toggle PDF"));
  await waitFor(() =>
    expect(rpc.mock.calls.filter(([op]) => op === "document")).toHaveLength(2),
  );
});

it("applying extraction settings refreshes every open Library and preserves variants", async () => {
  const custom = {
    ...project.cards[0],
    id: "variant",
    name: "Saved variant",
    origin: "custom" as const,
    parentID: "source",
  };
  const outside = {
    ...project.cards[0],
    id: "outside",
    name: "Outside table",
    tex: "y",
  };
  project.cards.push(outside, custom);
  const other = { ...structuredClone(project), id: "other", root: "/other" };
  availableProjects.push(other);
  const base = rpc.getMockImplementation()!;
  rpc.mockImplementation((operation, payload) => {
    if (operation === "extraction_settings")
      return Promise.resolve({
        value: {
          schemaVersion: 1,
          kind: "mathlidelux.extraction",
          ignoreMathInTables: false,
          excludedLatexEnvironments: [],
          excludedTypstFunctions: [],
        },
        revision: "extraction",
      });
    if (operation === "settings" && payload.file === "extraction.json")
      return Promise.resolve("filtered");
    if (operation === "scan") {
      const current = availableProjects.find((p) => p.id === payload.project)!;
      return Promise.resolve({
        ...current,
        cards: current.cards.filter((c) => c.id === "outside"),
        extractionHiddenCards: current.cards.filter((c) => c.id === "source"),
        extractionRevision: "filtered",
      });
    }
    return base(operation, payload);
  });
  await openProject();
  fireEvent.click(screen.getByRole("button", { name: "Projects" }));
  fireEvent.click(await screen.findByRole("button", { name: "Open other" }));
  await screen.findByTitle("New equation");
  await waitFor(() => expect(document.documentElement.inert).toBe(false));
  fireEvent.click(screen.getByTitle("Settings"));
  fireEvent.click(screen.getByRole("button", { name: "Extraction" }));
  const toggle = await screen.findByRole("checkbox", {
    name: "Ignore math in tables",
  });
  await waitFor(() =>
    expect((toggle as HTMLInputElement).disabled).toBe(false),
  );
  fireEvent.click(toggle);
  fireEvent.click(screen.getByRole("button", { name: "Apply and rescan" }));
  await waitFor(() =>
    expect(
      availableProjects.every((p) => p.extractionRevision === "filtered"),
    ).toBe(true),
  );
  await waitFor(() => expect(document.documentElement.inert).toBe(false));
  for (const p of availableProjects) {
    expect(p.cards).toEqual([outside, custom]);
    expect(p.extractionHiddenCards?.[0].id).toBe("source");
  }
  expect(
    rpc.mock.calls
      .filter(([op]) => op === "scan")
      .map(([, payload]) => payload.project),
  ).toEqual(["p", "other"]);
  expect(rpc.mock.calls.some(([op]) => op === "save_file")).toBe(false);
  fireEvent.click(screen.getByRole("button", { name: "Close" }));
  rpc.mockClear();
  fireEvent.click(screen.getByTitle("/synthetic"));
  await waitFor(() =>
    expect(
      rpc.mock.calls.some(
        ([op, payload]) =>
          op === "render" &&
          payload.project === "p" &&
          payload.cards.some((c: Card) => c.id === "outside"),
      ),
    ).toBe(true),
  );
  expect(rpc.mock.calls.some(([op]) => op === "scan")).toBe(false);
});

const MAC_USER_AGENT =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko)";
function addCards() {
  for (const [id, name] of [
    ["second", "Second"],
    ["third", "Third"],
  ])
    project.cards.push({
      id,
      name,
      tex: id,
      kind: "block",
      origin: "source",
      sourceOrder: project.cards.length,
      status: "pending",
    } as Card);
}
function exportCalls() {
  return rpc.mock.calls.filter(([operation]) => operation === "export");
}

it("exports the card whose context menu opened even when macOS delivers a Ctrl+click after the contextmenu", async () => {
  const userAgent = vi
    .spyOn(navigator, "userAgent", "get")
    .mockReturnValue(MAC_USER_AGENT);
  try {
    addCards();
    await openProject();
    const card = screen.getByRole("button", { name: "Second" });
    fireEvent.contextMenu(card);
    fireEvent.click(card, { ctrlKey: true });
    expect(card.getAttribute("aria-selected")).toBe("true");
    fireEvent.click(screen.getByRole("button", { name: "Export Image…" }));
    await waitFor(() => expect(exportCalls()).toHaveLength(1));
    expect(exportCalls()[0][1]).toMatchObject({ ids: ["second"] });
  } finally {
    userAgent.mockRestore();
  }
});

it("keeps Ctrl as the selection toggle outside macOS", async () => {
  addCards();
  await openProject();
  const second = screen.getByRole("button", { name: "Second" });
  const third = screen.getByRole("button", { name: "Third" });
  fireEvent.click(second);
  fireEvent.click(third, { ctrlKey: true });
  expect(second.getAttribute("aria-selected")).toBe("true");
  expect(third.getAttribute("aria-selected")).toBe("true");
  fireEvent.click(third, { ctrlKey: true });
  expect(third.getAttribute("aria-selected")).toBe("false");
});

it("keeps Cmd as the selection toggle on macOS", async () => {
  const userAgent = vi
    .spyOn(navigator, "userAgent", "get")
    .mockReturnValue(MAC_USER_AGENT);
  try {
    addCards();
    await openProject();
    const second = screen.getByRole("button", { name: "Second" });
    const third = screen.getByRole("button", { name: "Third" });
    fireEvent.click(second);
    fireEvent.click(third, { metaKey: true });
    expect(third.getAttribute("aria-selected")).toBe("true");
    fireEvent.click(third, { metaKey: true });
    expect(third.getAttribute("aria-selected")).toBe("false");
  } finally {
    userAgent.mockRestore();
  }
});

it("exports the right-clicked card and names the file after it when several cards are selected", async () => {
  addCards();
  await openProject();
  const second = screen.getByRole("button", { name: "Second" });
  const third = screen.getByRole("button", { name: "Third" });
  fireEvent.click(second);
  fireEvent.click(third, { shiftKey: true });
  expect(second.getAttribute("aria-selected")).toBe("true");
  expect(third.getAttribute("aria-selected")).toBe("true");
  fireEvent.contextMenu(third);
  fireEvent.click(screen.getByRole("button", { name: "Export Image…" }));
  await waitFor(() => expect(exportCalls()).toHaveLength(1));
  expect(save).toHaveBeenCalledWith(
    expect.objectContaining({ defaultPath: "Third.svg" }),
  );
  expect(exportCalls()[0][1]).toMatchObject({ ids: ["third"] });
});

it("reports an empty selection instead of opening the save dialog or calling export", async () => {
  addCards();
  await openProject();
  fireEvent.keyDown(document.body, {
    key: "s",
    shiftKey: true,
    metaKey: !isWindows(),
    ctrlKey: isWindows(),
  });
  expect(await screen.findByText("Select an equation or figure.")).toBeTruthy();
  expect(save).not.toHaveBeenCalled();
  expect(exportCalls()).toHaveLength(0);
});
