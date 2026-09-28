// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { registerWindowFlush } from "./persistence";
import { installWindowPersistence } from "./windowPersistence";

type Callback = (event: { payload: unknown }) => unknown;
const native = vi.hoisted(() => ({
  handlers: new Map<string, Callback>(),
  rpc: vi.fn(),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (name: string, callback: Callback) => {
    native.handlers.set(name, callback);
    return () => native.handlers.delete(name);
  }),
}));
vi.mock("./api", () => ({ rpc: native.rpc }));

const cleanup: (() => void)[] = [];
const report = vi.fn();
async function emit(name: string, payload: unknown) {
  await native.handlers.get(name)?.({ payload });
}
function deferred() {
  let resolve!: () => void;
  const promise = new Promise<void>((done) => (resolve = done));
  return { promise, resolve };
}
beforeEach(async () => {
  native.rpc.mockReset().mockResolvedValue(null);
  report.mockReset();
  cleanup.push(await installWindowPersistence(report));
});
afterEach(() => {
  for (const stop of cleanup.splice(0).reverse()) stop();
  expect(document.documentElement.inert).toBe(false);
});

it("keeps the saved window inert while a second window's save is delayed", async () => {
  const secondWindow = deferred();
  let secondFinished = false;
  const savingSecond = secondWindow.promise.then(() => {
    secondFinished = true;
  });
  // A native acknowledgement returns normally even while a second window saves.
  native.rpc.mockImplementation(async () => {
    expect(secondFinished).toBe(false);
    return null;
  });
  const flush = vi.fn(async () => {});
  cleanup.push(registerWindowFlush("main", flush));
  await emit("flush-window", { token: 1 });
  expect(flush).toHaveBeenCalledOnce();
  expect(native.rpc).toHaveBeenCalledWith("window_flush_complete", {
    token: 1,
    error: "",
  });
  expect(document.documentElement.inert).toBe(true);
  secondWindow.resolve();
  await savingSecond;
  // Successful Quit deliberately has no resume event: input stays blocked to exit.
  expect(document.documentElement.inert).toBe(true);
});

it("releases the already saved window when another window fails or times out", async () => {
  await emit("flush-window", { token: 2 });
  expect(document.documentElement.inert).toBe(true);
  await emit("flush-window-finished", { token: 2 });
  expect(document.documentElement.inert).toBe(false);
  await emit("persistence-error", { message: "saving timed out" });
  expect(report).toHaveBeenCalledWith({ message: "saving timed out" });
});

it("does not unlock a newer quit when an earlier close is cancelled", async () => {
  await emit("flush-window", { token: 3 });
  await emit("flush-window", { token: 4 });
  await emit("flush-window-finished", { token: 3 });
  expect(document.documentElement.inert).toBe(true);
  await emit("flush-window-finished", { token: 4 });
  expect(document.documentElement.inert).toBe(false);
});

it("ignores duplicate flushes and delayed flush events for a cancelled request", async () => {
  const flush = vi.fn(async () => {});
  cleanup.push(registerWindowFlush("main", flush));
  await emit("flush-window", { token: 5 });
  await emit("flush-window", { token: 5 });
  expect(flush).toHaveBeenCalledOnce();
  await emit("flush-window-finished", { token: 5 });
  await emit("flush-window", { token: 5 });
  expect(flush).toHaveBeenCalledOnce();
  expect(document.documentElement.inert).toBe(false);
});

it("keeps an in-flight save locked after cancellation until its flush finishes", async () => {
  const writing = deferred();
  cleanup.push(registerWindowFlush("main", () => writing.promise));
  const flush = emit("flush-window", { token: 6 });
  await emit("flush-window-finished", { token: 6 });
  expect(document.documentElement.inert).toBe(true);
  writing.resolve();
  await flush;
  expect(document.documentElement.inert).toBe(false);
});

it("reports failed acknowledgement transport but waits for native cancellation", async () => {
  native.rpc.mockRejectedValue(Error("bridge unavailable"));
  await emit("flush-window", { token: 7 });
  expect(report).toHaveBeenCalledWith(Error("bridge unavailable"));
  expect(document.documentElement.inert).toBe(true);
  await emit("flush-window-finished", { token: 7 });
  expect(document.documentElement.inert).toBe(false);
});

it("reports a failed local flush and resumes only when native cancels the request", async () => {
  cleanup.push(
    registerWindowFlush("main", async () => {
      throw Error("disk full");
    }),
  );
  await emit("flush-window", { token: 8 });
  expect(native.rpc).toHaveBeenCalledWith("window_flush_complete", {
    token: 8,
    error: "Error: Error: disk full",
  });
  expect(document.documentElement.inert).toBe(true);
  await emit("flush-window-finished", { token: 8 });
  expect(document.documentElement.inert).toBe(false);
});
