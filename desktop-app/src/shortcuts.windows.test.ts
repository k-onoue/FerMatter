import { afterAll, beforeAll, expect, it, vi } from "vitest";
import type { Binding, Keybindings } from "./types";
import shortcutRules from "../tests/fixtures/shortcut-rules.json";

let shortcuts: typeof import("./shortcuts");
beforeAll(async () => {
  // Test the Windows webview signal independently of the test runner's host.
  // Defaults are created at module load, so select Win32 before importing.
  vi.spyOn(navigator, "platform", "get").mockReturnValue("Win32");
  vi.resetModules();
  shortcuts = await import("./shortcuts");
});
afterAll(() => {
  vi.restoreAllMocks();
  vi.resetModules();
});

const binding = (key: string, extra: Partial<Binding> = {}): Binding => ({
  key,
  command: true,
  control: false,
  option: false,
  shift: false,
  ...extra,
});
const keys = (bindings: Record<string, Binding> = {}): Keybindings => ({
  schemaVersion: 1,
  kind: "mathlidelux.keybindings",
  bindings,
});
const event = (key: string, extra: KeyboardEventInit = {}) =>
  ({
    key,
    ctrlKey: true,
    metaKey: false,
    altKey: false,
    shiftKey: false,
    isComposing: false,
    ...extra,
  }) as KeyboardEvent;

it.each(shortcutRules)("validates the Rust fixture on Win32: $name", (rule) => {
  const cleared = (rule as { cleared?: string[] }).cleared ?? [];
  const validate = () =>
    shortcuts.validateBindings(
      keys({
        ...Object.fromEntries(cleared.map((id) => [id, binding("")])),
        [rule.command]: rule.binding as Binding,
      }),
    );
  if (rule.valid) expect(validate).not.toThrow();
  else expect(validate).toThrow();
});

it("dispatches physical Ctrl/Alt and distinguishes the Windows modifier", () => {
  expect(shortcuts.commandForEvent(event("s"), keys())).toBe("saveEquation");
  expect(shortcuts.commandForEvent(event("e", { altKey: true }), keys())).toBe(
    "projectSource",
  );
  expect(
    shortcuts.commandForEvent(
      event("s", { ctrlKey: false, metaKey: true }),
      keys(),
    ),
  ).toBeUndefined();
  const saved = keys({
    saveEquation: binding("s", { command: false, control: true }),
  });
  expect(() => shortcuts.validateBindings(saved)).not.toThrow();
  expect(
    shortcuts.commandForEvent(
      event("s", { ctrlKey: false, metaKey: true }),
      saved,
    ),
  ).toBe("saveEquation");
  expect(shortcuts.commandForEvent(event("s"), saved)).toBeUndefined();
  expect(shortcuts.bindingKeys(binding("e", { option: true }))).toEqual([
    "Alt",
    "Ctrl",
    "E",
  ]);
  expect(shortcuts.bindingKeys(saved.bindings.saveEquation)).toEqual([
    "Win",
    "S",
  ]);
});

it("keeps Ctrl+Tab and Ctrl+Shift+Tab editable without the Windows key", () => {
  expect(() => shortcuts.validateBindings(keys())).not.toThrow();
  expect(shortcuts.commandForEvent(event("Tab"), keys())).toBe("nextSourceTab");
  expect(
    shortcuts.commandForEvent(event("Tab", { shiftKey: true }), keys()),
  ).toBe("previousSourceTab");
  expect(
    shortcuts.commandForEvent(
      event("Tab", { ctrlKey: false, metaKey: true }),
      keys(),
    ),
  ).toBeUndefined();
  expect(() =>
    shortcuts.validateBindings(keys({ open: binding("\t") })),
  ).toThrow("Duplicate shortcut");
  const saved = keys({ nextSourceTab: binding(""), open: binding("\t") });
  const before = JSON.stringify(saved);
  expect(() => shortcuts.validateBindings(saved)).not.toThrow();
  expect(shortcuts.commandForEvent(event("Tab"), saved)).toBe("open");
  expect(
    shortcuts.commandForEvent(event("Tab", { shiftKey: true }), saved),
  ).toBe("previousSourceTab");
  expect(JSON.stringify(saved)).toBe(before);
});

it("preserves saved overrides and clears only conflicting implicit defaults", () => {
  const saved = keys({
    zoomOut: binding("-"),
    fitPDF: binding("0"),
    open: binding("=", { control: true }),
  });
  const before = JSON.stringify(saved);
  expect(() => shortcuts.validateBindings(saved)).not.toThrow();
  expect(shortcuts.effectiveBindings(saved).zoomWindowOut.key).toBe("");
  expect(shortcuts.commandForEvent(event("-"), saved)).toBe("zoomOut");
  expect(shortcuts.commandForEvent(event("=", { metaKey: true }), saved)).toBe(
    "open",
  );
  expect(
    shortcuts.commandForEvent(event("-", { isComposing: true }), saved),
  ).toBeUndefined();
  expect(JSON.stringify(saved)).toBe(before);
});
