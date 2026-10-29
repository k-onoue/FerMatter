import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { test } from "node:test";
import {
  releaseDownloadUrl,
  verifyReleaseDownload,
} from "../verify-release-download.mjs";

const bytes = Buffer.from("Synthetic DMG bytes");
const config = {
  releaseAvailable: true,
  releaseNotarized: false,
  releaseRepository: "example/releases",
  releaseTag: "v1.0-preview.1",
  assetName: "FerMatter.dmg",
  releaseSha256: createHash("sha256").update(bytes).digest("hex"),
};

test("anonymous download must match the reviewed release bytes", async () => {
  const url = releaseDownloadUrl(config);
  const result = await verifyReleaseDownload(config, async (requested) => {
    assert.equal(requested, url);
    return new Response(bytes);
  });
  assert.deepEqual(result, {
    url,
    sha256: config.releaseSha256,
    bytes: bytes.length,
  });
});

test("changed or unavailable release bytes block publication", async () => {
  await assert.rejects(
    verifyReleaseDownload(config, async () => new Response("Replacement DMG")),
    /differs from the reviewed value/,
  );
  await assert.rejects(
    verifyReleaseDownload(
      config,
      async () => new Response(null, { status: 404 }),
    ),
    /Cannot download the public macOS release/,
  );
  await assert.rejects(
    verifyReleaseDownload(
      config,
      async () => new Response(null, { status: 200 }),
    ),
    /Cannot download the public macOS release/,
  );
  await assert.rejects(
    verifyReleaseDownload(config, async () => new Response("")),
    /Public macOS release is empty/,
  );
  await assert.rejects(
    verifyReleaseDownload(config, async () => {
      throw new Error("anonymous fetch failed");
    }),
    /anonymous fetch failed/,
  );
});

test("enabled release needs a reviewed hash", async () => {
  await assert.rejects(
    verifyReleaseDownload({ ...config, releaseSha256: "" }, async () => {
      throw new Error("The network must not be reached");
    }),
    /reviewed SHA-256/,
  );
  assert.equal(
    await verifyReleaseDownload(
      { ...config, releaseAvailable: false },
      async () => {
        throw new Error("The network must not be reached");
      },
    ),
    null,
  );
});

test("malformed release flags and non-DMG assets block publication", async () => {
  for (const invalid of [
    { releaseAvailable: "false" },
    { releaseNotarized: "false" },
    { releaseTag: 42 },
    { assetName: "notes.txt" },
  ]) {
    await assert.rejects(
      verifyReleaseDownload({ ...config, ...invalid }, async () => {
        throw new Error("The network must not be reached");
      }),
      /Boolean|DMG asset/,
    );
  }
});
