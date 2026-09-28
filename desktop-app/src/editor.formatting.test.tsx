// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
} from "@testing-library/react";
const { state, formatSource } = vi.hoisted(() => ({
  state: { editors: [] as any[], models: new Map<string, any>() },
  formatSource: vi.fn(),
}));
vi.mock("./formatting", () => ({ formatSource }));
vi.mock("monaco-editor/editor/editor.worker.js?worker", () => ({
  default: class {},
}));
vi.mock("monaco-editor", () => ({
  languages: {
    register: vi.fn(),
    setMonarchTokensProvider: vi.fn(),
    setLanguageConfiguration: vi.fn(),
    registerCompletionItemProvider: vi.fn(),
  },
  KeyMod: { CtrlCmd: 1, Shift: 2, Alt: 4 },
  KeyCode: { KeyF: 8, KeyS: 16, Backspace: 32 },
  Uri: {
    from: (parts: any) => ({
      ...parts,
      toString: () => parts.authority + parts.path,
    }),
  },
  editor: {
    getModel: (uri: any) => state.models.get(uri.toString()),
    createModel: (initial: string, _language: string, uri: any) => {
      let value = initial,
        version = 1,
        disposed = false;
      const listeners = new Set<() => void>();
      const model = {
        getValue: () => value,
        getVersionId: () => version,
        isDisposed: () => disposed,
        getFullModelRange: () => ({}),
        write: (text: string) => {
          value = text;
          version++;
          for (const callback of listeners) callback();
        },
        onDidChangeContent: (callback: () => void) => {
          listeners.add(callback);
          return { dispose: () => listeners.delete(callback) };
        },
        dispose: () => {
          disposed = true;
          state.models.delete(uri.toString());
        },
      };
      state.models.set(uri.toString(), model);
      return model;
    },
    create: (_node: unknown, { model }: any) => {
      const editor = {
        getModel: () => model,
        pushUndoStop: vi.fn(),
        executeEdits: vi.fn((_key, edits) => model.write(edits[0].text)),
        focus: vi.fn(),
        dispose: vi.fn(),
        updateOptions: vi.fn(),
        saveViewState: () => null,
        restoreViewState: vi.fn(),
        onDidChangeCursorPosition: () => ({ dispose() {} }),
        onDidFocusEditorText: () => ({ dispose() {} }),
        addAction: vi.fn(),
        addCommand: vi.fn(),
      };
      state.editors.push(editor);
      return editor;
    },
  },
}));
import { CodeEditor, type EditorProps } from "./editor";
import { flushWindow } from "./persistence";
const props: EditorProps = {
  project: "synthetic",
  path: "main.typ",
  value: "#let x=(1,2)",
  format: "typst",
  onChange: vi.fn(),
};
let resolve!: (text: string) => void;
beforeEach(() => {
  state.editors.length = 0;
  state.models.clear();
  vi.clearAllMocks();
  formatSource.mockImplementation(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
});
afterEach(() => cleanup());
it("applies formatting as one undoable edit using the usual change/save route", async () => {
  render(<CodeEditor {...props} />);
  fireEvent.click(screen.getByRole("button", { name: "Format Source" }));
  expect(formatSource.mock.calls[0][0]).toEqual({
    source: props.value,
    language: "typst",
    mode: "document",
  });
  await act(async () => resolve("#let x = (1, 2)"));
  const editor = state.editors[0];
  expect(editor.getModel().getValue()).toBe("#let x = (1, 2)");
  expect(editor.pushUndoStop).toHaveBeenCalledTimes(2);
  expect(props.onChange).toHaveBeenCalledWith("#let x = (1, 2)");
  expect(editor.executeEdits).toHaveBeenCalledOnce();
});
it("keeps a newer edit when an obsolete formatter finishes", async () => {
  render(<CodeEditor {...props} />);
  fireEvent.click(screen.getByRole("button", { name: "Format Source" }));
  act(() => state.editors[0].getModel().write("newer typing"));
  expect(formatSource.mock.calls[0][1].aborted).toBe(true);
  await act(async () => resolve("obsolete result"));
  expect(state.editors[0].getModel().getValue()).toBe("newer typing");
  expect(state.editors[0].executeEdits).not.toHaveBeenCalled();
});
it("does not steal focus back after asynchronous formatting", async () => {
  render(<CodeEditor {...props} />);
  fireEvent.click(screen.getByRole("button", { name: "Format Source" }));
  const editor = state.editors[0];
  editor.focus.mockClear();
  // The user may now be typing in another split pane while this request runs.
  await act(async () => resolve("#let x = (1, 2)"));
  expect(editor.focus).not.toHaveBeenCalled();
});
it("cancels format work during window save/close and file changes", async () => {
  const view = render(<CodeEditor {...props} />);
  fireEvent.click(screen.getByRole("button", { name: "Format Source" }));
  await act(() => flushWindow());
  expect(formatSource.mock.calls[0][1].aborted).toBe(true);
  await act(async () => resolve("late result"));
  fireEvent.click(screen.getByRole("button", { name: "Format Source" }));
  view.rerender(<CodeEditor {...props} path="other.typ" value="other file" />);
  await act(async () => resolve("old file result"));
  expect(formatSource.mock.calls[1][1].aborted).toBe(true);
  expect(state.editors[1].getModel().getValue()).toBe("other file");
});
it("leaves text unchanged on failure and excludes non-source input modes", async () => {
  formatSource.mockRejectedValue(new Error("Typst has syntax errors."));
  const view = render(<CodeEditor {...props} />);
  await act(async () =>
    fireEvent.click(screen.getByRole("button", { name: "Format Source" })),
  );
  expect(screen.getByRole("alert").textContent).toContain("syntax errors");
  expect(state.editors[0].getModel().getValue()).toBe(props.value);
  view.rerender(<CodeEditor {...props} formatMode={false} />);
  expect(screen.queryByRole("button", { name: "Format Source" })).toBeNull();
});
