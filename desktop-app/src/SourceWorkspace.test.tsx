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
import { SourceWorkspace } from "./SourceWorkspace";
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
