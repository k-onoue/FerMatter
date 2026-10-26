import { afterEach, expect, it, vi } from "vitest";
import { fileManagerName, primaryModifier, shortcutHint } from "./platform";
import type { Keybindings } from "./types";

afterEach(() => {
  vi.restoreAllMocks();
  vi.resetModules();
});

it("names Finder on macOS and a generic file manager on Windows and Linux", () => {
  expect(
    fileManagerName(
      "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko)",
    ),
  ).toBe("Finder");
  expect(
    fileManagerName(
      "Mozilla/5.0 (X11; Ubuntu; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko)",
    ),
  ).toBe("File Manager");
  expect(
    fileManagerName(
      "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko)",
    ),
  ).toBe("File Manager");
});

it("uses Ctrl for Windows shortcuts while preserving the extra modifier", async () => {
  vi.spyOn(navigator, "platform", "get").mockReturnValue("Win32");
  vi.resetModules();
  const { commandForEvent, bindingKeys, validateBindings } =
    await import("./shortcuts");
  const keys: Keybindings = {
    schemaVersion: 1,
    kind: "mathlidelux.keybindings",
    bindings: {},
  };
  const event = (key: string, extra = {}) =>
    ({
      key,
      ctrlKey: true,
      metaKey: false,
      shiftKey: false,
      altKey: false,
      isComposing: false,
      ...extra,
    }) as KeyboardEvent;
  expect(() => validateBindings(keys)).not.toThrow();
  expect(commandForEvent(event("s"), keys)).toBe("saveEquation");
  expect(commandForEvent(event("Tab"), keys)).toBe("nextSourceTab");
  expect(commandForEvent(event("+", { shiftKey: true }), keys)).toBe(
    "zoomWindowIn",
  );
  expect(
    commandForEvent(event("+", { metaKey: true, shiftKey: true }), keys),
  ).toBe("zoomIn");
  expect(
    commandForEvent(event("s", { isComposing: true }), keys),
  ).toBeUndefined();
  expect(primaryModifier(event("s"))).toBe(true);
  expect(shortcutHint("Render (⌘↵)")).toBe("Render (Ctrl+Enter)");
  expect(
    bindingKeys({
      key: "e",
      command: true,
      control: false,
      option: true,
      shift: false,
    }),
  ).toEqual(["Alt", "Ctrl", "E"]);
});

it("retains macOS shortcut labels and modifiers", () => {
  vi.spyOn(navigator, "platform", "get").mockReturnValue("MacIntel");
  expect(primaryModifier({ ctrlKey: true, metaKey: false })).toBe(false);
  expect(shortcutHint("Copy (⌘C)")).toBe("Copy (⌘C)");
});
