import { expect, it } from "vitest";
import {
  commandForEvent,
  effectiveBindings,
  validateBindings,
} from "./shortcuts";
import type { Binding, Keybindings } from "./types";

const keys = (bindings: Record<string, Binding> = {}): Keybindings => ({
  schemaVersion: 1,
  kind: "mathlidelux.keybindings",
  bindings,
});
const binding = (key: string, options: Partial<Binding> = {}): Binding => ({
  key,
  command: true,
  control: false,
  option: false,
  shift: false,
  ...options,
});
const event = (key: string, options: Partial<KeyboardEvent> = {}) =>
  ({
    key,
    metaKey: true,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    isComposing: false,
    ...options,
  }) as KeyboardEvent;

it("zooms the window with equals, shifted plus, keypad plus, minus and zero", () => {
  expect(commandForEvent(event("="), keys())).toBe("zoomWindowIn");
  expect(commandForEvent(event("=", { shiftKey: true }), keys())).toBe(
    "zoomWindowIn",
  );
  expect(commandForEvent(event("+", { shiftKey: true }), keys())).toBe(
    "zoomWindowIn",
  );
  expect(commandForEvent(event("+"), keys())).toBe("zoomWindowIn");
  expect(commandForEvent(event(";", { shiftKey: true }), keys())).toBe(
    "zoomWindowIn",
  );
  expect(commandForEvent(event("-", { shiftKey: true }), keys())).toBe(
    "zoomWindowIn",
  );
  expect(commandForEvent(event("-"), keys())).toBe("zoomWindowOut");
  expect(commandForEvent(event("0"), keys())).toBe("resetWindowZoom");
  expect(
    commandForEvent(event("+", { isComposing: true }), keys()),
  ).toBeUndefined();
  expect(
    commandForEvent(event("-", { metaKey: false }), keys()),
  ).toBeUndefined();
});

it("keeps PDF zoom separate and honors custom and cleared zoom assignments", () => {
  expect(
    commandForEvent(event("+", { ctrlKey: true, shiftKey: true }), keys()),
  ).toBe("zoomIn");
  expect(commandForEvent(event("-", { ctrlKey: true }), keys())).toBe(
    "zoomOut",
  );
  expect(commandForEvent(event("0", { ctrlKey: true }), keys())).toBe("fitPDF");
  const custom = keys({
    zoomWindowIn: binding("j"),
    zoomWindowOut: binding(""),
  });
  expect(commandForEvent(event("j"), custom)).toBe("zoomWindowIn");
  expect(
    commandForEvent(event("+", { shiftKey: true }), custom),
  ).toBeUndefined();
  expect(commandForEvent(event("-"), custom)).toBeUndefined();
});

it("preserves legacy overrides without invalidating the entire shortcut file", () => {
  const legacy = keys({
    zoomOut: binding("-"),
    fitPDF: binding("0"),
    open: binding("=", { control: true }),
  });
  expect(() => validateBindings(legacy)).not.toThrow();
  expect(effectiveBindings(legacy).zoomWindowOut.key).toBe("");
  expect(effectiveBindings(legacy).resetWindowZoom.key).toBe("");
  expect(effectiveBindings(legacy).zoomIn.key).toBe("");
  expect(commandForEvent(event("-"), legacy)).toBe("zoomOut");
  expect(commandForEvent(event("0"), legacy)).toBe("fitPDF");
  const explicit = keys({ zoomOut: binding("-"), zoomWindowOut: binding("-") });
  expect(() => validateBindings(explicit)).toThrow("Duplicate shortcut");
});

it("does not let the plus alias steal an explicitly assigned shifted shortcut", () => {
  const custom = keys({ open: binding("=", { shift: true }) });
  expect(commandForEvent(event("+", { shiftKey: true }), custom)).toBe("open");
  expect(commandForEvent(event("="), custom)).toBe("zoomWindowIn");
  const jisCustom = keys({ open: binding(";", { shift: true }) });
  expect(commandForEvent(event(";", { shiftKey: true }), jisCustom)).toBe(
    "open",
  );
});
