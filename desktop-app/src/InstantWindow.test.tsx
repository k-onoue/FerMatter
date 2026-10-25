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
import InstantWindow from "./InstantWindow";
import { defaults } from "./appearance";
import {
  initialInstant,
  instantPendingKey,
  type InstantDraft,
  type InstantSession,
} from "./instant";
import type { Card } from "./types";

const { rpc, events, save, drag } = vi.hoisted(() => ({
  rpc: vi.fn(),
  events: new Map<string, Set<() => void>>(),
  save: vi.fn(),
  drag: vi.fn(),
}));
vi.mock("./api", () => ({
  rpc,
  uid: () => crypto.randomUUID(),
  projectName: () => "Synthetic",
  asset: async (path: string) => "data:image/svg+xml," + path,
}));
vi.mock("./editor", () => ({
  setProjectCompletions: vi.fn(),
  CodeEditor: ({
    value,
    onChange,
    path,
  }: {
    value: string;
    onChange: (value: string) => void;
    path: string;
  }) => (
    <textarea
      aria-label={
        path.startsWith("template-") ? "Template source" : "Equation source"
      }
      value={value}
      onChange={(event) => onChange(event.target.value)}
    />
  ),
}));
vi.mock("./useAppearance", () => ({ useAppearance: () => {} }));
vi.mock("./useDesktopBackground", () => ({ useDesktopBackground: () => null }));
vi.mock("./windowPersistence", () => ({
  installWindowPersistence: async () => () => {},
}));
vi.mock("./artifactLease", async (original) => ({
  ...(await original<typeof import("./artifactLease")>()),
  useCardArtifacts: () => {},
}));
vi.mock("@crabnebula/tauri-plugin-drag", () => ({ startDrag: drag }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: async (name: string, callback: () => void) => {
    if (!events.has(name)) events.set(name, new Set());
    events.get(name)!.add(callback);
    return () => events.get(name)?.delete(callback);
  },
}));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    onFocusChanged: async () => () => {},
    setAlwaysOnTop: async () => {},
  }),
}));
vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({ setZoom: async () => {} }),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ save, open: vi.fn() }));

type Result = { card: Card; path: string };
let session: InstantSession;
let saved: InstantDraft[];
let remembered: InstantDraft[];
let pending: { token: string; draft: InstantDraft }[];
let saveDraft: ((draft: InstantDraft) => Promise<void>) | undefined;
let rememberDraft: (() => Promise<void>) | undefined;
let renderDraft: ((draft: InstantDraft) => Promise<Result>) | undefined;

function result(draft: InstantDraft): Result {
  const path = `/synthetic/${draft.source}-${draft.color.slice(1)}.${draft.exportFormat}`;
  return {
    path,
    card: {
      id: "instant",
      tex: draft.source,
      svgPath: path,
      pngPath: path,
    } as Card,
  };
}
beforeEach(() => {
  vi.stubGlobal(
    "PointerEvent",
    class extends MouseEvent {
      readonly pointerId: number;
      constructor(type: string, init: PointerEventInit = {}) {
        super(type, init);
        this.pointerId = init.pointerId ?? 1;
      }
    },
  );
  session = {
    schemaVersion: 1,
    draft: { ...initialInstant(), source: "x" },
    history: [],
  };
  saved = [];
  remembered = [];
  pending = [];
  saveDraft = undefined;
  rememberDraft = undefined;
  renderDraft = undefined;
  localStorage.clear();
  events.clear();
  save.mockReset().mockResolvedValue("/synthetic/export.svg");
  drag.mockReset().mockResolvedValue(undefined);
  rpc.mockReset().mockImplementation(async (operation, payload) => {
    if (operation === "startup")
      return {
        settings: {
          value: { appearance: "light", opacity: 1 },
          revision: "settings",
        },
        theme: { value: defaults, revision: "theme" },
        keybindings: {
          value: {
            schemaVersion: 1,
            kind: "mathlidelux.keybindings",
            bindings: {},
          },
          revision: "keys",
        },
      };
    if (operation === "instant_configuration")
      return {
        value: {
          schemaVersion: 1,
          kind: "mathlidelux.instant",
          templates: [session.draft!.template],
          defaultTemplateID: session.draft!.template.id,
          fontSize: 24,
          exportFormat: "svg",
          color: "#000000",
          inputMode: "block",
        },
        revision: "config",
      };
    if (operation === "instant_session") {
      if (payload.action === "save") {
        await saveDraft?.(payload.draft);
        saved.push(structuredClone(payload.draft));
        session.draft = structuredClone(payload.draft);
      }
      if (payload.action === "remember") {
        await rememberDraft?.();
        remembered.push(structuredClone(payload.draft));
      }
      if (
        ["save", "remember"].includes(payload.action) &&
        payload.draft.source.trim()
      ) {
        const draft = structuredClone(payload.draft);
        const existing = session.history.find(
          (item) => item.id === draft.entryID,
        );
        if (existing) existing.draft = draft;
        else session.history.unshift({ id: draft.entryID, draft });
      }
      return structuredClone(session);
    }
    if (operation === "instant_import_pending") {
      if (payload?.token) {
        expect(pending[0]?.token).toBe(payload.token);
        pending.shift();
      }
      return pending[0] || null;
    }
    if (operation === "instant_render")
      return renderDraft ? renderDraft(payload.draft) : result(payload.draft);
    return null;
  });
});
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

async function openInstant() {
  render(<InstantWindow />);
  await screen.findByRole("textbox", { name: "Equation source" });
  await waitFor(() =>
    expect(
      (screen.getByRole("button", { name: "Copy image" }) as HTMLButtonElement)
        .disabled,
    ).toBe(false),
  );
}
async function importDraft(draft: InstantDraft) {
  pending.push({ token: "restored", draft });
  await act(async () => {
    for (const event of events.get("instant-import-pending") || []) event();
  });
}
function color(name: string) {
  if (!screen.queryByRole("dialog", { name: "Equation color picker" }))
    fireEvent.click(screen.getByRole("button", { name: "Equation color" }));
  fireEvent.click(screen.getByRole("button", { name }));
}
function previewPath() {
  return screen
    .getByRole("img", { name: "Rendered equation" })
    .getAttribute("src");
}

it("opens demo project setup without replacing the Instant draft or creating projects", async () => {
  await openInstant();
  const draft = structuredClone(session.draft);
  rpc.mockClear();
  fireEvent.click(screen.getByRole("button", { name: "Demo projects" }));
  await waitFor(() => expect(rpc).toHaveBeenCalledWith("projects_open"));
  expect(session.draft).toEqual(draft);
  expect(
    rpc.mock.calls.some(([operation]) =>
      ["complete_project_setup", "templates", "settings"].includes(operation),
    ),
  ).toBe(false);
});

it("color changes re-render the preview and copy/export the new output", async () => {
  await openInstant();
  color("Red");
  expect(
    (screen.getByRole("button", { name: "Copy image" }) as HTMLButtonElement)
      .disabled,
  ).toBe(true);
  await waitFor(() => expect(previewPath()).toContain("dc2626"));
  expect(saved.at(-1)?.color).toBe("#dc2626");
  const path = result({ ...session.draft!, color: "#dc2626" }).path;
  fireEvent.click(screen.getByRole("button", { name: "Copy image" }));
  await waitFor(() =>
    expect(rpc).toHaveBeenCalledWith("copy_images", { paths: [path] }),
  );
  await waitFor(() =>
    expect(
      (
        screen.getByRole("button", {
          name: "Export image",
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(false),
  );
  fireEvent.click(screen.getByRole("button", { name: "Export image" }));
  await waitFor(() =>
    expect(rpc).toHaveBeenCalledWith("instant_export", {
      path,
      destination: "/synthetic/export.svg",
    }),
  );
  expect(saved.at(-1)?.color).toBe("#dc2626");
});

it("shows autosaved equations on reopen, updates one item, and duplicates without changing the original", async () => {
  await openInstant();
  expect(
    screen.getByRole("complementary", { name: "Instant library" }),
  ).toBeTruthy();
  const firstID = session.draft!.entryID;
  color("Blue");
  fireEvent.keyDown(document.body, { key: "Escape" });
  await waitFor(() => expect(session.history[0].draft.color).toBe("#2563eb"));
  expect(session.history).toHaveLength(1);
  fireEvent.click(screen.getByRole("button", { name: "Duplicate equation" }));
  await waitFor(() => expect(session.history).toHaveLength(2));
  const secondID = session.draft!.entryID;
  expect(secondID).not.toBe(firstID);
  color("Red");
  fireEvent.keyDown(document.body, { key: "Escape" });
  await waitFor(() => expect(session.history[0].draft.color).toBe("#dc2626"));
  expect(session.history[1].draft.color).toBe("#2563eb");
  fireEvent.click(screen.getByRole("button", { name: "New equation" }));
  await waitFor(() => expect(session.draft!.source).toBe(""));
  expect(session.history).toHaveLength(2);
  cleanup();
  render(<InstantWindow />);
  await screen.findByRole("textbox", { name: "Equation source" });
  fireEvent.change(screen.getByRole("textbox", { name: "Search equations" }), {
    target: { value: "x" },
  });
  const items = screen.getAllByRole("button", { name: /x\s*LaTeX/ });
  fireEvent.click(items[1]);
  await waitFor(() => expect(session.draft!.entryID).toBe(firstID));
  expect(session.draft!.color).toBe("#2563eb");
  await waitFor(() => expect(previewPath()).toContain("2563eb"));
  expect(rpc.mock.calls.some(([op]) => op === "copy_images")).toBe(false);
});

it("restores and renders an image automatically while preserving the previous draft and waiting to acknowledge its save", async () => {
  await openInstant();
  let finishSave!: () => void;
  saveDraft = (draft) =>
    draft.source === "restored"
      ? new Promise((resolve) => {
          finishSave = resolve;
        })
      : Promise.resolve();
  await importDraft({
    ...initialInstant(),
    source: "restored",
    color: "#2563eb",
    requiresRender: true,
  });
  await waitFor(() => expect(previewPath()).toContain("restored-2563eb"));
  expect(pending).toHaveLength(1);
  expect(remembered[0].source).toBe("x");
  await act(async () => finishSave());
  await waitFor(() => expect(pending).toHaveLength(0));
  expect(previewPath()).toContain("restored-2563eb");
  expect(saved.at(-1)?.requiresRender).toBe(false);
  expect(
    (screen.getByRole("button", { name: "Copy image" }) as HTMLButtonElement)
      .disabled,
  ).toBe(false);
  expect(rpc.mock.calls.some(([op]) => op === "save_file")).toBe(false);
});

it("ignores an obsolete color render that finishes after a newer one", async () => {
  await openInstant();
  let finishRed!: (value: Result) => void;
  renderDraft = (draft) =>
    draft.color === "#dc2626"
      ? new Promise((resolve) => {
          finishRed = resolve;
        })
      : Promise.resolve(result(draft));
  color("Red");
  await waitFor(() => expect(finishRed).toBeTypeOf("function"));
  color("Blue");
  await waitFor(() => expect(previewPath()).toContain("2563eb"));
  await act(async () =>
    finishRed(result({ ...session.draft!, color: "#dc2626" })),
  );
  expect(previewPath()).toContain("2563eb");
  expect(rpc).toHaveBeenCalledWith("cancel", { key: "render" });
  fireEvent.click(screen.getByRole("button", { name: "Copy image" }));
  await waitFor(() =>
    expect(rpc).toHaveBeenCalledWith("copy_images", {
      paths: [result({ ...session.draft!, color: "#2563eb" }).path],
    }),
  );
});

it("migrates an older imported recovery draft to automatic preview before display", async () => {
  localStorage.setItem(
    instantPendingKey(""),
    JSON.stringify({
      ...session.draft,
      source: "recovered",
      requiresRender: true,
    }),
  );
  await openInstant();
  expect(previewPath()).toContain("recovered");
  expect(saved[0]).toMatchObject({
    source: "recovered",
    requiresRender: false,
  });
  expect(localStorage.getItem(instantPendingKey(""))).toBeNull();
});

it("keeps newly typed text and leaves the import queued when history saving overlaps an edit", async () => {
  await openInstant();
  let finishHistory!: () => void;
  rememberDraft = () =>
    new Promise((resolve) => {
      finishHistory = resolve;
    });
  await importDraft({ ...initialInstant(), source: "incoming" });
  fireEvent.change(screen.getByRole("textbox", { name: "Equation source" }), {
    target: { value: "still typing" },
  });
  await act(async () => finishHistory());
  await screen.findByText(/SVG is waiting/);
  expect(
    (
      screen.getByRole("textbox", {
        name: "Equation source",
      }) as HTMLTextAreaElement
    ).value,
  ).toBe("still typing");
  expect(pending).toHaveLength(1);
  expect(saved.some((draft) => draft.source === "incoming")).toBe(false);
});

function pointerImage() {
  const image = screen.getByRole("img", { name: "Rendered equation" });
  image.setPointerCapture = vi.fn();
  image.hasPointerCapture = () => true;
  image.releasePointerCapture = vi.fn();
  return image;
}

it("starts a native file drag after pointer movement without an HTML image drag, and rearms after completion", async () => {
  await openInstant();
  const image = pointerImage();
  await waitFor(() => expect(image.style.cursor).toBe("grab"));
  const point = {
    pointerId: 1,
    button: 0,
    buttons: 1,
    clientX: 10,
    clientY: 20,
  };
  fireEvent.pointerDown(image, point);
  fireEvent.pointerMove(image, { ...point, clientX: 12 });
  expect(drag).not.toHaveBeenCalled();
  fireEvent.pointerMove(image, { ...point, clientX: 18 });
  expect(drag).toHaveBeenCalledTimes(1);
  expect(drag.mock.calls[0][0]).toEqual({
    item: [result(session.draft!).path],
    icon: result(session.draft!).path,
    mode: "copy",
  });
  expect(image.releasePointerCapture).toHaveBeenCalledWith(1);
  expect(image.draggable).toBe(false);
  fireEvent.pointerMove(image, { ...point, clientX: 30 });
  expect(drag).toHaveBeenCalledTimes(1);
  await act(async () => drag.mock.calls[0][1]({ result: "Dropped" }));
  await waitFor(() => expect(image.style.cursor).toBe("grab"));
  fireEvent.pointerDown(image, point);
  fireEvent.pointerMove(image, { ...point, clientX: 18 });
  expect(drag).toHaveBeenCalledTimes(2);
});

it("does not drag for a click, a released pointer, or a preview made obsolete by editing", async () => {
  await openInstant();
  const image = pointerImage();
  await waitFor(() => expect(image.style.cursor).toBe("grab"));
  const point = {
    pointerId: 1,
    button: 0,
    buttons: 1,
    clientX: 10,
    clientY: 20,
  };
  fireEvent.pointerDown(image, point);
  fireEvent.pointerUp(image, { ...point, buttons: 0 });
  fireEvent.pointerMove(image, { ...point, clientX: 30 });
  expect(drag).not.toHaveBeenCalled();
  fireEvent.pointerDown(image, point);
  fireEvent.pointerMove(image, { ...point, buttons: 0, clientX: 30 });
  expect(drag).not.toHaveBeenCalled();
  fireEvent.pointerDown(image, point);
  fireEvent.change(screen.getByRole("textbox", { name: "Equation source" }), {
    target: { value: "new source" },
  });
  fireEvent.pointerMove(image, { ...point, clientX: 30 });
  expect(drag).not.toHaveBeenCalled();
});
