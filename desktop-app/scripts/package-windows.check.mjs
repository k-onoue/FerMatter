import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { peMachine, preserveFile } from "./package-windows.mjs";

test("Windows packaging rejects malformed and truncated executables", () => {
  assert.throws(
    () => peMachine(Buffer.from("not an executable")),
    /DOS header/,
  );
  const bytes = Buffer.alloc(128);
  bytes.write("MZ");
  bytes.writeUInt32LE(80, 60);
  bytes.write("PE\0\0", 80);
  bytes.writeUInt16LE(0x8664, 84);
  assert.equal(peMachine(bytes), 0x8664);
  assert.throws(() => peMachine(bytes.subarray(0, 90)), /truncated/);
  bytes.writeUInt32LE(0xffffffff, 60);
  assert.throws(() => peMachine(bytes), /truncated/);
});

test("Windows packaging preserves existing release bytes", () => {
  const directory = mkdtempSync(join(tmpdir(), "fermatter-package-"));
  try {
    const path = join(directory, "preview.exe");
    preserveFile(path, Buffer.from("original"));
    preserveFile(path, Buffer.from("original"));
    assert.throws(
      () => preserveFile(path, Buffer.from("replacement")),
      /Refusing to replace/,
    );
    assert.equal(readFileSync(path, "utf8"), "original");
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
