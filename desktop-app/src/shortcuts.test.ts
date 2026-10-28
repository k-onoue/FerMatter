import { expect, it } from "vitest";
import {
  commandForEvent,
  effectiveBindings,
  validateBindings,
} from "./shortcuts";
import type { Binding, Keybindings } from "./types";
import shortcutRules from "../tests/fixtures/shortcut-rules.json";
import { isWindows } from "./platform";

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
const event = (key: string, options: Partial<KeyboardEvent> = {}) => {
  const logical = {
    key,
    metaKey: true,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    isComposing: false,
    ...options,
  };
  return {
    ...logical,
    metaKey: isWindows() ? logical.ctrlKey : logical.metaKey,
    ctrlKey: isWindows() ? logical.metaKey : logical.ctrlKey,
  } as KeyboardEvent;
};

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

it("dispatches Option combinations with the primary modifier unchanged", () => {
  expect(commandForEvent(event("\\"), keys())).toBe("splitSource");
  expect(commandForEvent(event("\\", { altKey: true }), keys())).toBe(
    "splitSourceDown",
  );
  expect(commandForEvent(event("e", { altKey: true }), keys())).toBe(
    "projectSource",
  );
  expect(
    commandForEvent(event("\\", { metaKey: false, altKey: true }), keys()),
  ).toBeUndefined();
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

// The same fixture drives the Rust validator in src-tauri/src/settings.rs.
// `cleared` lists commands whose default is emptied so the case cannot fail as
// a duplicate of that default.
it.each(shortcutRules)(
  "validates shortcuts like the Rust validator: $name",
  ({ command, binding: candidate, valid, ...rest }) => {
    const cleared = (rest as { cleared?: string[] }).cleared ?? [];
    const validate = () =>
      validateBindings(
        keys({
          ...Object.fromEntries(cleared.map((id) => [id, binding("")])),
          [command]: candidate as Binding,
        }),
      );
    if (valid) expect(validate).not.toThrow();
    else expect(validate).toThrow();
  },
);
