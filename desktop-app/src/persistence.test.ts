// @vitest-environment jsdom
import { describe, expect, it, vi } from "vitest";
import {
  SnapshotWriter,
  registerWindowFlush,
  flushWindow,
  lockWindowInteraction,
  runWhenWindowInteractive,
} from "./persistence";

describe("persistence", () => {
  it("runs focus immediately when interactive and only after the last nested lease releases", () => {
    const focus = vi.fn(() =>
      expect(document.documentElement.inert).not.toBe(true),
    );
    runWhenWindowInteractive(focus);
    expect(focus).toHaveBeenCalledTimes(1);
    const transition = lockWindowInteraction();
    const importing = lockWindowInteraction();
    runWhenWindowInteractive(focus);
    transition();
    expect(focus).toHaveBeenCalledTimes(1);
    importing();
    expect(focus).toHaveBeenCalledTimes(2);
    importing();
    expect(focus).toHaveBeenCalledTimes(2);
  });
  it("cancels obsolete focus callbacks and pauses remaining callbacks when a new lease is acquired", () => {
    const release = lockWindowInteraction();
    const obsolete = vi.fn();
    const current = vi.fn();
    const cancel = runWhenWindowInteractive(obsolete);
    cancel();
    cancel();
    let releaseNext!: () => void;
    runWhenWindowInteractive(() => {
      releaseNext = lockWindowInteraction();
    });
    runWhenWindowInteractive(current);
    release();
    expect(obsolete).not.toHaveBeenCalled();
    expect(current).not.toHaveBeenCalled();
    releaseNext();
    expect(current).toHaveBeenCalledTimes(1);
  });
  it("reports failure, retries the latest snapshot and accepts later writes", async () => {
    const saved: number[] = [];
    let failing = true;
    const writer = new SnapshotWriter<number>(async (value) => {
      if (failing) throw Error("disk full");
      saved.push(value);
    });
    await expect(writer.save(1)).rejects.toThrow("disk full");
    await expect(writer.flush()).rejects.toThrow("disk full");
    failing = false;
    await writer.save(2);
    await writer.flush();
    expect(saved).toEqual([2]);
  });
  it("waits for edits arriving during an in-flight write", async () => {
    let release!: () => void;
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    const saved: number[] = [];
    const writer = new SnapshotWriter<number>(async (value) => {
      if (value === 1) await gate;
      saved.push(value);
    });
    void writer.save(1);
    const flush = writer.flush();
    void writer.save(2);
    release();
    await flush;
    expect(saved.at(-1)).toBe(2);
  });
  it("preserves other drafts when one flush fails and can retry", async () => {
    const saved: string[] = [];
    const a = registerWindowFlush("a", async () => {
      throw Error("permission denied");
    });
    const b = registerWindowFlush("b", async () => {
      saved.push("b");
    });
    await expect(flushWindow()).rejects.toThrow("permission denied");
    expect(saved).toEqual(["b"]);
    a();
    await flushWindow();
    b();
  });
  it("an import can flush editors without recursively awaiting its own drain", async () => {
    let saves = 0;
    const editor = registerWindowFlush("editor", async () => {
      saves++;
    });
    const imports = registerWindowFlush("workspace-imports", async () => {
      await flushWindow(["workspace-imports"]);
    });
    await flushWindow();
    expect(saves).toBe(2);
    editor();
    imports();
  });
});
