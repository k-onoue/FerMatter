// @vitest-environment jsdom
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { ExtractionSettings } from "./ExtractionSettings";
const { rpc } = vi.hoisted(() => ({ rpc: vi.fn() }));
vi.mock("./api", () => ({ rpc }));
vi.mock("./editor", () => ({ CodeEditor: () => null }));
afterEach(cleanup);
const value = {
  schemaVersion: 1,
  kind: "mathlidelux.extraction",
  ignoreMathInTables: false,
  excludedLatexEnvironments: [],
  excludedTypstFunctions: [],
  extensions: { keep: true },
};
it("a conflicting save preserves the draft until Reload accepts the external file", async () => {
  const onApplied = vi.fn().mockResolvedValue(undefined);
  rpc.mockReset().mockImplementation(async (operation) => {
    if (operation === "extraction_settings") return { value, revision: "old" };
    if (operation === "settings")
      throw Error("Settings changed on disk. Reload the file before saving.");
    if (operation === "reload_settings")
      return {
        value: { ...value, excludedLatexEnvironments: ["results"] },
        revision: "new",
      };
  });
  render(<ExtractionSettings onApplied={onApplied} />);
  const toggle = screen.getByRole("checkbox") as HTMLInputElement;
  await waitFor(() => expect(toggle.disabled).toBe(false));
  fireEvent.click(toggle);
  fireEvent.click(screen.getByText("Apply and rescan"));
  await screen.findByRole("alert");
  expect(toggle.checked).toBe(true);
  expect(onApplied).not.toHaveBeenCalled();
  expect(rpc).toHaveBeenCalledWith("settings", {
    file: "extraction.json",
    value: { ...value, ignoreMathInTables: true },
    revision: "old",
  });
  fireEvent.click(screen.getByText("Reload"));
  await waitFor(() => expect(onApplied).toHaveBeenCalledWith("new"));
  expect(toggle.checked).toBe(false);
});
it("last-valid fallback cannot overwrite invalid external JSON", async () => {
  rpc.mockReset().mockResolvedValue({ value, revision: "" });
  render(<ExtractionSettings onApplied={vi.fn()} />);
  await screen.findByRole("alert");
  expect((screen.getByRole("checkbox") as HTMLInputElement).disabled).toBe(
    true,
  );
  expect(
    (screen.getByText("Apply and rescan") as HTMLButtonElement).disabled,
  ).toBe(true);
  expect(rpc.mock.calls.map(([op]) => op)).toEqual(["extraction_settings"]);
});
