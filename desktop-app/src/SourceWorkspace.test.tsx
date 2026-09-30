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
import type { EditorProps } from "./editor";
import type { Buffer, Project } from "./types";
import { flushWindow, lockWindowInteraction } from "./persistence";
import { SourceWorkspace, type SourceSession } from "./SourceWorkspace";
const { rpc } = vi.hoisted(() => ({ rpc: vi.fn() }));
vi.mock("./api", () => ({ rpc, uid: () => crypto.randomUUID() }));
vi.mock("./editor", () => ({
  setProjectCompletions: vi.fn(),
  CodeEditor: (props: EditorProps) => (
    <textarea
      aria-label="Source editor"
      data-model={props.modelKey}
      value={props.value}
      readOnly={props.readOnly}
      onChange={(event) => props.onChange(event.target.value)}
    />
  ),
}));
vi.mock("@crabnebula/tauri-plugin-drag", () => ({ startDrag: vi.fn() }));
const project = {
  id: "project",
  main: "main.typ",
  root: "/synthetic",
  format: "typst",
  cards: [],
  outline: [],
  candidates: ["main.typ"],
} as unknown as Project;
let disk: Buffer;
let draftWrites: Record<string, Buffer>[];
let saveImpl: ((buffer: Buffer) => Promise<Buffer>) | undefined;
let moveImpl: (() => Promise<unknown>) | undefined;
let sidebar: HTMLDivElement;
beforeEach(() => {
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      disconnect() {}
    },
  );
  sidebar = document.createElement("div");
  sidebar.id = "source-sidebar";
  document.body.append(sidebar);
  disk = {
    path: "main.typ",
    text: "$ original $",
    revision: "one",
    bom: false,
    newline: "\n",
  };
  draftWrites = [];
  saveImpl = undefined;
  moveImpl = undefined;
  rpc
    .mockReset()
    .mockImplementation(
      async (operation: string, payload: Record<string, unknown>) => {
        if (operation === "read_drafts") return { buffers: {}, warnings: [] };
        if (operation === "tree")
          return [{ path: disk.path, directory: false }];
        if (operation === "read_file") return { ...disk };
        if (operation === "save_drafts") {
          draftWrites.push(
            structuredClone(payload.value as Record<string, Buffer>),
          );
          return null;
        }
        if (operation === "save_file") {
          const buffer = payload.buffer as Buffer;
          const result = saveImpl
            ? await saveImpl(buffer)
            : { ...buffer, revision: "saved" };
          disk = { ...result };
          return result;
        }
        if (operation === "file_operation") {
          await moveImpl?.();
          disk = { ...disk, path: String(payload.destination) };
          return { main: disk.path, candidates: [disk.path] };
        }
        return null;
      },
    );
});
afterEach(() => {
  cleanup();
  sidebar.remove();
  vi.unstubAllGlobals();
  document.documentElement.inert = false;
});
async function workspace(autoSave = false) {
  const onProjectChanged = vi.fn();
  const onError = vi.fn();
  render(
    <SourceWorkspace
      project={project}
      autoSave={autoSave}
      onDrafts={() => {}}
      onError={onError}
      ask={async () => "moved.typ"}
      onSaved={() => {}}
      beforeMutation={async () => {}}
      onProjectChanged={onProjectChanged}
      onCursor={() => {}}
    />,
  );
  const editor = await screen.findByRole("textbox", { name: "Source editor" });
  return { editor, onProjectChanged, onError };
}
it("copies caret previews at the default or decimal override and can return to Default", async () => {
  const original = rpc.getMockImplementation()!;
  rpc.mockImplementation(async (op, payload) => {
    if (op === "source_preview")
      return {
        id: "eq",
        tex: "x",
        kind: "inline",
        status: "ready",
        sourceFontSize: 11,
      };
    if (op === "preview_export") return "export.svg";
    return original(op, payload);
  });
  render(
    <SourceWorkspace
      project={project}
      defaultFontSize={30}
      autoSave={false}
      session={{
        buffers: { "main.typ": { ...disk, dirty: true } },
        layout: { id: "pane", tabs: ["main.typ"], active: "main.typ" },
        activePane: "pane",
        expanded: new Set(),
        treeMode: "files",
        query: "",
        caret: { file: "main.typ", line: 1, offset: 3 },
      }}
      onDrafts={() => {}}
      onError={vi.fn()}
      ask={async () => null}
      onSaved={() => {}}
      beforeMutation={async () => {}}
      onProjectChanged={() => {}}
      onCursor={() => {}}
    />,
  );
  const input = await screen.findByRole("spinbutton", {
    name: "Output font size",
  });
  expect((input as HTMLInputElement).value).toBe("30");
  fireEvent.click(screen.getByTitle("Copy image"));
  await waitFor(() =>
    expect(rpc).toHaveBeenCalledWith(
      "preview_export",
      expect.objectContaining({ fontSize: 30 }),
    ),
  );
  fireEvent.change(input, { target: { value: "31.25" } });
  fireEvent.blur(input);
  fireEvent.click(screen.getByTitle("Copy image"));
  await waitFor(() =>
    expect(rpc).toHaveBeenCalledWith(
      "preview_export",
      expect.objectContaining({ fontSize: 31.25 }),
    ),
  );
  fireEvent.click(
    screen.getByRole("button", { name: "Use default output font size" }),
  );
  expect((input as HTMLInputElement).value).toBe("30");
});
it("flushes the mounted manual editor immediately, before its 700ms timer", async () => {
  const { editor } = await workspace();
  fireEvent.change(editor, { target: { value: "$ latest $" } });
  await act(async () => {
    await flushWindow();
  });
  expect(draftWrites.at(-1)?.["main.typ"].text).toBe("$ latest $");
  expect(rpc.mock.calls.some(([op]) => op === "save_file")).toBe(false);
});
it("a delayed Source save cannot clear newer typing when closing", async () => {
  const { editor } = await workspace(true);
  let finish!: () => void;
  saveImpl = (buffer) =>
    new Promise((resolve) => {
      finish = () => resolve({ ...buffer, revision: "saved-first" });
    });
  fireEvent.change(editor, { target: { value: "$ first $" } });
  act(() =>
    window.dispatchEvent(
      new CustomEvent("source-command", { detail: "saveEquation" }),
    ),
  );
  await waitFor(() => expect(finish).toBeDefined());
  fireEvent.change(editor, { target: { value: "$ latest $" } });
  await act(async () => {
    const closing = flushWindow();
    saveImpl = undefined;
    finish();
    await closing;
  });
  expect(disk.text).toBe("$ latest $");
  expect(draftWrites.at(-1)).toEqual({});
});
it("closing during a rename waits for remapped drafts and preserves model identity", async () => {
  const { editor, onProjectChanged } = await workspace();
  fireEvent.change(editor, { target: { value: "$ unsaved $" } });
  const model = editor.getAttribute("data-model");
  let finish!: () => void;
  moveImpl = () =>
    new Promise((resolve) => {
      finish = () => resolve(null);
    });
  const row = document.querySelector(".tree-row")!;
  fireEvent.contextMenu(row);
  fireEvent.click(screen.getByRole("button", { name: "Rename…" }));
  await waitFor(() => expect(finish).toBeDefined());
  await act(async () => {
    const closing = flushWindow();
    finish();
    await closing;
  });
  expect(onProjectChanged).toHaveBeenCalledWith({
    main: "moved.typ",
    candidates: ["moved.typ"],
  });
  expect(draftWrites.at(-1)?.["moved.typ"].text).toBe("$ unsaved $");
  expect(draftWrites.at(-1)?.["main.typ"]).toBeUndefined();
  expect(
    screen
      .getByRole("textbox", { name: "Source editor" })
      .getAttribute("data-model"),
  ).toBe(model);
});
it("failed draft RPC rejects window close while the mounted editor keeps its text", async () => {
  const { editor } = await workspace();
  fireEvent.change(editor, { target: { value: "$ retained $" } });
  const base = rpc.getMockImplementation()!;
  rpc.mockImplementation((operation, ...args) =>
    operation === "save_drafts"
      ? Promise.reject(Error("disk full"))
      : base(operation, ...args),
  );
  await act(async () => {
    await expect(flushWindow()).rejects.toThrow("disk full");
  });
  expect((editor as HTMLTextAreaElement).value).toBe("$ retained $");
  rpc.mockImplementation(base);
  await act(async () => {
    await flushWindow();
  });
  expect(draftWrites.at(-1)?.["main.typ"].text).toBe("$ retained $");
});

it("nested mutation/close locks cannot re-enable editing before the final save completes", () => {
  const mutation = lockWindowInteraction();
  const close = lockWindowInteraction();
  mutation();
  expect(document.documentElement.inert).toBe(true);
  mutation();
  expect(document.documentElement.inert).toBe(true);
  close();
  expect(document.documentElement.inert).toBe(false);
});

it("restores Source panes and unsaved buffers with the same model after tab suspension", async () => {
  let session: SourceSession | undefined;
  const props = {
    project,
    autoSave: false,
    onDrafts: () => {},
    onError: vi.fn(),
    ask: async () => null,
    onSaved: () => {},
    beforeMutation: async () => {},
    onProjectChanged: () => {},
    onCursor: () => {},
    onSession: (value: SourceSession) => {
      session = value;
    },
  };
  const first = render(<SourceWorkspace {...props} />);
  const editor = await screen.findByRole("textbox", { name: "Source editor" });
  fireEvent.change(editor, { target: { value: "$ draft survives $" } });
  act(() =>
    window.dispatchEvent(
      new CustomEvent("source-command", { detail: "splitSourceDown" }),
    ),
  );
  await act(() => flushWindow());
  const model = editor.getAttribute("data-model");
  first.unmount();
  expect(session?.buffers["main.typ"].dirty).toBe(true);
  render(<SourceWorkspace {...props} session={session} />);
  const editors = await screen.findAllByRole("textbox", {
    name: "Source editor",
  });
  expect(editors).toHaveLength(2);
  for (const restored of editors) {
    expect((restored as HTMLTextAreaElement).value).toBe("$ draft survives $");
    expect(restored.getAttribute("data-model")).toBe(model);
  }
  expect(rpc.mock.calls.filter(([op]) => op === "read_drafts")).toHaveLength(1);
  expect(session?.buffers["main.typ"].recovered).not.toBe(true);
});

it("keeps the restored active file instead of replaying old PDF navigation, but accepts a new navigation", async () => {
  const session: SourceSession = {
    buffers: {
      "main.typ": { ...disk, dirty: true, modelKey: "main-model" },
      "other.typ": {
        ...disk,
        path: "other.typ",
        text: "$ other $",
        dirty: true,
        modelKey: "other-model",
      },
    },
    layout: {
      id: "pane",
      tabs: ["main.typ", "other.typ"],
      active: "other.typ",
    },
    activePane: "pane",
    expanded: new Set(),
    treeMode: "files",
    query: "",
    previewSize: 0,
    caret: { file: "other.typ", line: 3, offset: 9 },
    navigationNonce: 7,
  };
  const props = {
    project,
    session,
    autoSave: false,
    onDrafts: () => {},
    onError: vi.fn(),
    ask: async () => null,
    onSaved: () => {},
    beforeMutation: async () => {},
    onProjectChanged: () => {},
    onCursor: () => {},
  };
  const view = render(
    <SourceWorkspace
      {...props}
      navigate={{ path: "main.typ", line: 10, nonce: 7 }}
    />,
  );
  expect(
    (
      await screen.findByRole("textbox", { name: "Source editor" })
    ).getAttribute("data-model"),
  ).toBe("other-model");
  view.rerender(
    <SourceWorkspace
      {...props}
      navigate={{ path: "main.typ", line: 10, nonce: 8 }}
    />,
  );
  await waitFor(() =>
    expect(
      screen
        .getByRole("textbox", { name: "Source editor" })
        .getAttribute("data-model"),
    ).toBe("main-model"),
  );
});

it("does not suspend an incomplete initial file read as an empty restored session", async () => {
  let session: SourceSession | undefined;
  const props = {
    project,
    autoSave: false,
    onDrafts: () => {},
    onError: vi.fn(),
    ask: async () => null,
    onSaved: () => {},
    beforeMutation: async () => {},
    onProjectChanged: () => {},
    onCursor: () => {},
    onSession: (value: SourceSession) => {
      session = value;
    },
  };
  const base = rpc.getMockImplementation()!;
  let finish!: (value: Buffer) => void;
  rpc.mockImplementation((operation, ...args) => {
    if (operation === "tree") return Promise.resolve([]);
    if (operation === "read_file")
      return new Promise((resolve) => {
        finish = resolve;
      });
    return base(operation, ...args);
  });
  const first = render(<SourceWorkspace {...props} />);
  await waitFor(() => expect(finish).toBeDefined());
  await act(() => flushWindow());
  first.unmount();
  expect(session).toBeUndefined();
  rpc.mockImplementation(base);
  render(<SourceWorkspace {...props} session={session} />);
  await act(async () => {
    finish({ ...disk, text: "$ obsolete read $" });
  });
  const editor = await screen.findByRole("textbox", { name: "Source editor" });
  expect((editor as HTMLTextAreaElement).value).toBe("$ original $");
});

it("applies initial PDF navigation only after recovered drafts and the default file finish opening", async () => {
  const base = rpc.getMockImplementation()!;
  let finishDrafts!: () => void;
  let finishMain!: () => void;
  rpc.mockImplementation((operation, payload) => {
    if (operation === "tree") return Promise.resolve([]);
    if (operation === "read_drafts")
      return new Promise((resolve) => {
        finishDrafts = () => resolve({ buffers: {}, warnings: [] });
      });
    if (operation === "read_file" && payload.path === "main.typ")
      return new Promise((resolve) => {
        finishMain = () => resolve({ ...disk });
      });
    if (operation === "read_file")
      return Promise.resolve({
        ...disk,
        path: payload.path,
        text: "$ navigation target $",
      });
    return base(operation, payload);
  });
  render(
    <SourceWorkspace
      project={project}
      autoSave={false}
      onDrafts={() => {}}
      onError={vi.fn()}
      ask={async () => null}
      onSaved={() => {}}
      beforeMutation={async () => {}}
      onProjectChanged={() => {}}
      onCursor={() => {}}
      navigate={{ path: "target.typ", line: 8, nonce: 1 }}
    />,
  );
  await act(async () => {
    finishDrafts();
  });
  await waitFor(() => expect(finishMain).toBeDefined());
  await act(async () => {
    finishMain();
  });
  await waitFor(() =>
    expect(
      (
        screen.getByRole("textbox", {
          name: "Source editor",
        }) as HTMLTextAreaElement
      ).value,
    ).toBe("$ navigation target $"),
  );
});

it("refreshes clean suspended buffers without replacing newer typing", async () => {
  let session: SourceSession | undefined;
  const props = {
    project,
    autoSave: false,
    onDrafts: () => {},
    onError: vi.fn(),
    ask: async () => null,
    onSaved: () => {},
    beforeMutation: async () => {},
    onProjectChanged: () => {},
    onCursor: () => {},
    onSession: (value: SourceSession) => {
      session = value;
    },
  };
  const first = render(<SourceWorkspace {...props} />);
  await screen.findByRole("textbox", { name: "Source editor" });
  await act(() => flushWindow());
  first.unmount();
  disk = { ...disk, text: "$ external change $", revision: "external" };
  const second = render(<SourceWorkspace {...props} session={session} />);
  await waitFor(() =>
    expect(
      (
        screen.getByRole("textbox", {
          name: "Source editor",
        }) as HTMLTextAreaElement
      ).value,
    ).toBe("$ external change $"),
  );
  second.unmount();
  const base = rpc.getMockImplementation()!;
  const reads: ((value: Buffer) => void)[] = [];
  rpc.mockImplementation((operation, ...args) =>
    operation === "read_file"
      ? new Promise((resolve) => reads.push(resolve))
      : base(operation, ...args),
  );
  render(<SourceWorkspace {...props} session={session} />);
  const editor = await screen.findByRole("textbox", { name: "Source editor" });
  fireEvent.change(editor, { target: { value: "$ typed after return $" } });
  await act(async () => {
    for (const done of reads) done({ ...disk, text: "$ obsolete read $" });
  });
  expect((editor as HTMLTextAreaElement).value).toBe("$ typed after return $");
});
