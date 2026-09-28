import { describe, expect, it } from "vitest";
import {
  SnapshotWriter,
  registerWindowFlush,
  flushWindow,
} from "./persistence";

describe("persistence", () => {
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
});
