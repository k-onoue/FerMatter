import { afterEach, beforeEach, expect, it, vi } from "vitest";
const { workers, rpc } = vi.hoisted(() => ({
  workers: [] as any[],
  rpc: vi.fn(),
}));
vi.mock("./api", () => ({ rpc }));
vi.mock("./formatting.worker?worker", () => ({
  default: class {
    onmessage?: (event: unknown) => void;
    onerror?: () => void;
    terminate = vi.fn();
    postMessage = vi.fn();
    constructor() {
      workers.push(this);
    }
  },
}));
import { formatSource } from "./formatting";
import { FORMAT_LIMIT } from "./formattingTypes";
const request = { source: "x\r\ny", language: "latex", mode: "math" } as const;
beforeEach(() => {
  workers.length = 0;
  rpc.mockReset();
  vi.useFakeTimers();
});
afterEach(() => vi.useRealTimers());
it("runs LaTeX off-thread and retires its worker after a result", async () => {
  const promise = formatSource(request, new AbortController().signal);
  expect(workers[0].postMessage).toHaveBeenCalledWith(request);
  workers[0].onmessage({ data: { text: "x\ny\n" } });
  expect(await promise).toBe("x\r\ny");
  expect(workers[0].terminate).toHaveBeenCalledOnce();
});
it("terminates a cancelled LaTeX request and ignores a late callback", async () => {
  const controller = new AbortController();
  const promise = formatSource(request, controller.signal);
  controller.abort();
  workers[0].onmessage({ data: { text: "obsolete" } });
  await expect(promise).rejects.toMatchObject({ name: "AbortError" });
  expect(workers[0].terminate).toHaveBeenCalledOnce();
});
it("bounds worker time and rejects malformed responses without hanging", async () => {
  const promise = formatSource(request, new AbortController().signal);
  const assertion = expect(promise).rejects.toThrow("too long");
  await vi.advanceTimersByTimeAsync(10000);
  await assertion;
  expect(workers[0].terminate).toHaveBeenCalledOnce();
  const invalid = formatSource(request, new AbortController().signal);
  workers[1].onmessage({ data: {} });
  await expect(invalid).rejects.toThrow("invalid result");
});
it("cancels native Typst work with the same key and ignores late success", async () => {
  let finish!: (text: string) => void;
  rpc.mockImplementation((op) =>
    op === "cancel"
      ? Promise.resolve()
      : new Promise((resolve) => {
          finish = resolve;
        }),
  );
  const controller = new AbortController();
  const promise = formatSource(
    { ...request, language: "typst" },
    controller.signal,
  );
  const key = rpc.mock.calls[0][1].key;
  controller.abort();
  finish("stale");
  await expect(promise).rejects.toMatchObject({ name: "AbortError" });
  expect(rpc).toHaveBeenCalledWith("cancel", { key });
});
it("does not dispatch already-cancelled or oversized buffers", async () => {
  const controller = new AbortController();
  controller.abort();
  await expect(formatSource(request, controller.signal)).rejects.toMatchObject({
    name: "AbortError",
  });
  await expect(
    formatSource(
      { ...request, source: "é".repeat(FORMAT_LIMIT) },
      new AbortController().signal,
    ),
  ).rejects.toThrow("1 MB");
  expect(workers).toHaveLength(0);
  expect(rpc).not.toHaveBeenCalled();
});
it("keeps window flush pending until a cancelled native helper is reaped", async () => {
  const { flushWindow } = await import("./persistence");
  let finish!: (text: string) => void;
  rpc.mockImplementation((op) =>
    op === "cancel"
      ? Promise.resolve()
      : new Promise((resolve) => {
          finish = resolve;
        }),
  );
  const result = formatSource(
    { ...request, language: "typst" },
    new AbortController().signal,
  );
  const assertion = expect(result).rejects.toMatchObject({
    name: "AbortError",
  });
  let flushed = false;
  const flushing = flushWindow().then(() => {
    flushed = true;
  });
  await Promise.resolve();
  expect(flushed).toBe(false);
  expect(rpc).toHaveBeenCalledWith("cancel", { key: rpc.mock.calls[0][1].key });
  finish("ignored");
  await assertion;
  await flushing;
  expect(flushed).toBe(true);
});
