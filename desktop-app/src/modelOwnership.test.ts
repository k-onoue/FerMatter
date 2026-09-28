import { expect, it } from "vitest";
import { ModelOwnership } from "./modelOwnership";
it("keeps shared models/Undo until all panes release a closing project", () => {
  const disposed: string[] = [];
  const models = new ModelOwnership<string>((value) => disposed.push(value));
  const a = models.acquire("p", "stable-buffer", "undo stack", true);
  const b = models.acquire("p", "stable-buffer", "undo stack", true);
  a();
  expect(disposed).toEqual([]);
  models.releaseProject("p");
  expect(disposed).toEqual([]);
  b();
  expect(disposed).toEqual(["undo stack"]);
  expect(models.size).toBe(0);
});
it("does not grow across many completed projects or transient variant sessions", () => {
  const models = new ModelOwnership<object>(() => {});
  for (let i = 0; i < 1000; i++) {
    const release = models.acquire(String(i), String(i), {}, true);
    release();
    models.releaseProject(String(i));
    const variant = models.acquire(String(i), `variant-${i}`, {}, false);
    variant();
  }
  expect(models.size).toBe(0);
});
it("retains the same model during tab switches and rename by stable buffer identity", () => {
  const disposed: string[] = [];
  const models = new ModelOwnership<string>((value) => disposed.push(value));
  models.acquire("p", "buffer-1", "existing Undo", true)();
  const renamed = models.acquire("p", "buffer-1", "existing Undo", true);
  expect(models.size).toBe(1);
  renamed();
  expect(disposed).toEqual([]);
  models.releaseProject("p");
  expect(disposed).toEqual(["existing Undo"]);
});
