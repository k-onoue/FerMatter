#!/usr/bin/env node
// Check the anonymous bytes users receive before advertising a macOS release.
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { isMain } from "./lib/paths.mjs";

const MAX_DOWNLOAD_BYTES = 256 * 1024 * 1024;

export function releaseDownloadUrl(config) {
  const repository = config.releaseRepository;
  if (
    typeof repository !== "string" ||
    !/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repository) ||
    repository.split("/").some((part) => part === "." || part === "..")
  )
    throw new Error("Invalid release repository.");
  const tag = config.releaseTag;
  const asset = config.assetName;
  if (
    typeof tag !== "string" ||
    typeof asset !== "string" ||
    !/^[A-Za-z0-9._-]+$/.test(tag) ||
    !/^[A-Za-z0-9._-]+$/.test(asset) ||
    !/\.dmg$/i.test(asset) ||
    [tag, asset].some((part) => part === "." || part === "..")
  )
    throw new Error("Invalid macOS release tag or DMG asset.");
  return `https://github.com/${repository}/releases/download/${encodeURIComponent(tag)}/${encodeURIComponent(asset)}`;
}

export function validateReleaseConfig(config) {
  if (typeof config.releaseAvailable !== "boolean")
    throw new Error("macOS release availability must be a Boolean.");
  if (!config.releaseAvailable) return false;
  if (typeof config.releaseNotarized !== "boolean")
    throw new Error("macOS release notarization status must be a Boolean.");
  const expected = config.releaseSha256;
  if (typeof expected !== "string" || !/^[0-9a-f]{64}$/.test(expected))
    throw new Error("Published macOS release needs a reviewed SHA-256.");
  releaseDownloadUrl(config);
  return true;
}

export async function verifyReleaseDownload(config, fetcher = fetch) {
  if (!validateReleaseConfig(config)) return null;
  const expected = config.releaseSha256;
  const url = releaseDownloadUrl(config);
  const response = await fetcher(url, {
    redirect: "follow",
    signal: AbortSignal.timeout(120_000),
  });
  if (!response.ok || !response.body)
    throw new Error(
      `Cannot download the public macOS release (${response.status}).`,
    );
  const hash = createHash("sha256");
  let bytes = 0;
  for await (const chunk of response.body) {
    bytes += chunk.byteLength;
    if (bytes > MAX_DOWNLOAD_BYTES)
      throw new Error("Public macOS release exceeds the verification limit.");
    hash.update(chunk);
  }
  if (bytes === 0) throw new Error("Public macOS release is empty.");
  const actual = hash.digest("hex");
  if (actual !== expected)
    throw new Error(
      `Public macOS release SHA-256 differs from the reviewed value: ${actual}`,
    );
  return { url, sha256: actual, bytes };
}

if (isMain(import.meta.url)) {
  try {
    const config = JSON.parse(
      readFileSync(
        new URL("../site/src/data/site.json", import.meta.url),
        "utf8",
      ),
    );
    const result = await verifyReleaseDownload(config);
    console.log(
      result
        ? `Verified anonymous macOS DMG: ${result.sha256} (${result.bytes} bytes).`
        : "macOS download is disabled; no release asset to verify.",
    );
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
