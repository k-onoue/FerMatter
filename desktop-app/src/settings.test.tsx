// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { defaults } from "./appearance";
import { Settings } from "./settings";

vi.mock("./editor", () => ({ CodeEditor: () => null }));
vi.mock("./CompilerTools", () => ({ CompilerTools: () => null }));
afterEach(cleanup);

it("waits past a lone AltGr press when recording a shortcut", () => {
  const onKeys = vi.fn();
  const onError = vi.fn();
  render(
    <Settings
      settings={{
        appearance: "system",
        autoSave: true,
        opacity: 1,
        theme: defaults.id,
        pdfWidth: 500,
        projectView: "grid",
      }}
      themes={[]}
      theme={defaults}
      keys={{ schemaVersion: 1, kind: "mathlidelux.keybindings", bindings: {} }}
      onSettings={vi.fn()}
      onTheme={vi.fn()}
      onKeys={onKeys}
      onClose={vi.fn()}
      onReload={vi.fn()}
      onExtractionApplied={vi.fn()}
      onError={onError}
      defaultProjectDirectory="/synthetic/projects"
    />,
  );
  fireEvent.click(screen.getByRole("button", { name: "Shortcuts" }));
  const row = screen.getByText("Split Editor Down").closest(".shortcut-row");
  const recorder = row?.querySelector(".key-recorder") as HTMLElement;
  fireEvent.click(recorder);
  fireEvent.keyDown(recorder, { key: "AltGraph" });
  expect(onError).not.toHaveBeenCalled();
  expect(onKeys).not.toHaveBeenCalled();
  expect(recorder.textContent).toBe("Press keys…");
});

it("sorts theme choices without mutating shared themes and forwards appearance selections", () => {
  const themes = [
    { ...defaults, id: "zebra", name: "Zebra" },
    { ...defaults, id: "amber", name: "Amber" },
  ];
  Object.freeze(themes);
  const onTheme = vi.fn();
  const onSettings = vi.fn();
  render(
    <Settings
      settings={{
        appearance: "system",
        autoSave: true,
        opacity: 1,
        theme: defaults.id,
        pdfWidth: 500,
        projectView: "grid",
      }}
      themes={themes}
      theme={defaults}
      keys={{ schemaVersion: 1, kind: "mathlidelux.keybindings", bindings: {} }}
      onSettings={onSettings}
      onTheme={onTheme}
      onKeys={vi.fn()}
      onClose={vi.fn()}
      onReload={vi.fn()}
      onExtractionApplied={vi.fn()}
      onError={vi.fn()}
      defaultProjectDirectory="/synthetic/projects"
    />,
  );
  const select = screen.getByRole("combobox", {
    name: "Color Theme",
  }) as HTMLSelectElement;
  expect(Array.from(select.options, (option) => option.text)).toEqual([
    "Default",
    "Amber",
    "Zebra",
    "Custom",
  ]);
  expect(themes.map((theme) => theme.id)).toEqual(["zebra", "amber"]);
  fireEvent.change(select, { target: { value: "amber" } });
  expect(onTheme).toHaveBeenCalledWith(themes[1]);
  fireEvent.change(screen.getByRole("combobox", { name: "Appearance" }), {
    target: { value: "light" },
  });
  expect(onSettings).toHaveBeenCalledWith({ appearance: "light" });
});
