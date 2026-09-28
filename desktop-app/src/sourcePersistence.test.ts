import { expect, it } from "vitest";
import { flushSourceBuffers } from "./sourcePersistence";
import type { Buffer } from "./types";
const draft = (text: string): Buffer => ({
  path: "main.typ",
  text,
  dirty: true,
  revision: "revision",
  bom: false,
  newline: "\n",
});
it("protects edits within the debounce interval without saving manual-mode source", async () => {
  const buffers = { "main.typ": draft("just typed") };
  const records: unknown[] = [];
  await flushSourceBuffers({
    pending: () => [],
    buffers: () => buffers,
    saveDrafts: async (value) => {
      records.push(value);
    },
    save: async () => {
      throw Error("must not write source");
    },
    autoSave: false,
  });
  expect(records.at(-1)).toEqual(buffers);
});
it("waits for the in-flight save and retains newer typing during a slow draft write", async () => {
  let buffers = { "main.typ": draft("first") };
  let written = "";
  let first = true;
  await flushSourceBuffers({
    pending: () => [Promise.resolve()],
    buffers: () => buffers,
    saveDrafts: async (value) => {
      written = value["main.typ"].text;
      if (first) {
        first = false;
        buffers = { "main.typ": draft("latest") };
      }
    },
    save: async () => {},
    autoSave: false,
  });
  expect(written).toBe("latest");
});
it("keeps conflicts and recovered source as drafts without silently overwriting files", async () => {
  const buffers = {
    "conflict.typ": { ...draft("conflict"), error: "changed externally" },
    "recover.typ": { ...draft("recovered"), recovered: true },
  };
  let saved = 0;
  await flushSourceBuffers({
    pending: () => [],
    buffers: () => buffers,
    saveDrafts: async () => {},
    save: async () => {
      saved++;
    },
    autoSave: true,
  });
  expect(saved).toBe(0);
});
it("blocks close when recovery cannot be written but preserves drafts after a failed autosave", async () => {
  const buffers = { "main.typ": draft("latest") };
  const options = {
    pending: () => [],
    buffers: () => buffers,
    saveDrafts: async () => {
      throw Error("disk full");
    },
    save: async () => {},
    autoSave: true,
  };
  await expect(flushSourceBuffers(options)).rejects.toThrow("disk full");
  let preserved: unknown;
  await flushSourceBuffers({
    ...options,
    saveDrafts: async (value) => {
      preserved = value;
    },
    save: async () => {
      throw Error("permission denied");
    },
  });
  expect(preserved).toEqual(buffers);
});
