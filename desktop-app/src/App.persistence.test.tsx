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
const { rpc } = vi.hoisted(() => ({ rpc: vi.fn() }));
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
vi.mock("@tauri-apps/api/event", () => ({ listen: async () => () => {} }));
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
  }: {
    projects: Project[];
    onOpen: (p: Project) => void;
  }) => <button onClick={() => onOpen(projects[0])}>Open synthetic</button>,
}));
vi.mock("./VariantEditor", () => ({
  VariantEditor: ({
    editing,
    setEditing,
    onSave,
  }: {
    editing: Card;
    setEditing: (c: Card) => void;
    onSave: () => void;
  }) => (
    <section>
      <textarea
        aria-label="Variant source"
        value={editing.tex}
        onChange={(e) => setEditing({ ...editing, tex: e.target.value })}
      />
      <button onClick={onSave}>Save test variant</button>
    </section>
  ),
}));
let project: Project;
let saved: Project[];
let failSave: boolean;
let delaySave: ((value: Project) => Promise<void>) | undefined;
let scanImpl: (() => Promise<Project>) | undefined;
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
  failSave = false;
  delaySave = undefined;
  scanImpl = undefined;
  rpc
    .mockReset()
    .mockImplementation(
      async (operation: string, payload: Record<string, unknown>) => {
        if (operation === "startup")
          return {
            projects: [project],
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
            templates: { custom: [] },
          };
        if (operation === "open_project") return project;
        if (operation === "tree") return [];
        if (operation === "save_project") {
          if (failSave) throw Error("disk full");
          const next = structuredClone(payload.value as Project);
          await delaySave?.(next);
          saved.push(next);
          project = next;
          return null;
        }
        if (operation === "scan") return scanImpl ? scanImpl() : project;
        if (operation === "render") return [];
        return null;
      },
    );
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
}
function edit(text: string) {
  fireEvent.click(screen.getByTitle("New equation"));
  fireEvent.change(screen.getByRole("textbox", { name: "Variant source" }), {
    target: { value: text },
  });
}
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
