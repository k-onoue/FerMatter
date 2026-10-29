// @vitest-environment jsdom
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { UpdatesPanel, newestUpdate, type UpdateState } from "./Updates";
const { rpc, callbacks } = vi.hoisted(() => ({
  rpc: vi.fn(),
  callbacks: new Set<(event: { payload: unknown }) => void>(),
}));
vi.mock("./api", () => ({ rpc }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (_event, callback) => {
    callbacks.add(callback);
    return () => callbacks.delete(callback);
  }),
}));
afterEach(() => {
  cleanup();
  callbacks.clear();
});
const available: UpdateState = {
  revision: 1,
  currentVersion: "0.11.0",
  phase: "available",
  automatic: false,
  lastCheck: 0,
  version: "0.12.0",
  notes: "Fixes a synthetic example.",
  downloaded: 0,
  total: 100,
  error: null,
  installAvailable: true,
};
it("downloads only on request, requires a separate install action and retains a failed-save retry", async () => {
  rpc.mockReset().mockImplementation(async (operation) => {
    if (operation === "update_download")
      return { ...available, phase: "ready", revision: 2 };
    if (operation === "update_install")
      return {
        ...available,
        phase: "ready",
        revision: 3,
        error: "An editor could not save.",
      };
    return available;
  });
  render(<UpdatesPanel />);
  fireEvent.click(await screen.findByText("Download update"));
  await screen.findByText("Install and restart");
  expect(rpc.mock.calls.map(([op]) => op)).toEqual([
    "update_state",
    "update_download",
  ]);
  fireEvent.click(screen.getByText("Install and restart"));
  expect((await screen.findByRole("alert")).textContent).toContain(
    "could not save",
  );
  expect(
    (screen.getByText("Install and restart") as HTMLButtonElement).disabled,
  ).toBe(false);
});
it("cancellation returns to retry and late snapshots cannot resurrect progress", async () => {
  const downloading = {
    ...available,
    phase: "downloading" as const,
    revision: 4,
    downloaded: 30,
  };
  const cancelled = { ...available, revision: 5 };
  rpc
    .mockReset()
    .mockImplementation(async (operation) =>
      operation === "update_cancel" ? cancelled : downloading,
    );
  render(<UpdatesPanel />);
  fireEvent.click(await screen.findByText("Cancel"));
  await screen.findByText("Download update");
  expect(newestUpdate(cancelled, downloading)).toBe(cancelled);
  expect(rpc).toHaveBeenCalledWith("update_cancel", {});
});
it("automatic checks require explicit opt-in; source builds cannot install", async () => {
  rpc.mockReset().mockResolvedValue({ ...available, installAvailable: false });
  render(<UpdatesPanel />);
  const toggle = await screen.findByRole("checkbox");
  expect((toggle as HTMLInputElement).checked).toBe(false);
  expect(
    (screen.getByText("Download update") as HTMLButtonElement).disabled,
  ).toBe(true);
  fireEvent.click(toggle);
  await waitFor(() =>
    expect(rpc).toHaveBeenCalledWith("update_automatic", { enabled: true }),
  );
});
