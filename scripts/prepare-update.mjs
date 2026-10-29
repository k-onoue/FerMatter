#!/usr/bin/env node
// Prepare a local stable-update manifest. Does not upload artifacts or publish a feed.
import { createHash, createPublicKey, verify } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { ROOT, isMain } from "./lib/paths.mjs";

export const targets = {
  "darwin-aarch64-app": ".app.tar.gz",
  "windows-x86_64-nsis": ".exe",
  "linux-x86_64-deb": ".deb",
};
const stable = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const maximum = 1_000_000_000;
export function publicKey() {
  return JSON.parse(
    readFileSync(
      path.join(ROOT, "desktop-app/src-tauri/tauri.conf.json"),
      "utf8",
    ),
  ).plugins.updater.pubkey;
}
export function validateFeed(feed) {
  if (
    !stable.test(feed.version) ||
    !feed.platforms ||
    typeof feed.platforms !== "object" ||
    Array.isArray(feed.platforms)
  )
    throw Error("Expected a stable version and platform map.");
  const entries = Object.entries(feed.platforms);
  if ((feed.version === "0.0.0") !== (entries.length === 0))
    throw Error("Only the disabled 0.0.0 feed may have no platforms.");
  if (
    feed.notes !== undefined &&
    (typeof feed.notes !== "string" || feed.notes.length > 16000)
  )
    throw Error("Invalid update notes.");
  for (const [target, entry] of entries) {
    const suffix = targets[target];
    const url = new URL(entry.url);
    const prefix = `/FerMatter/fermatter.github.io/releases/download/v${feed.version}/`;
    const name = url.pathname.slice(prefix.length);
    if (
      !suffix ||
      url.origin !== "https://github.com" ||
      url.username ||
      url.password ||
      url.search ||
      url.hash ||
      !url.pathname.startsWith(prefix) ||
      !/^[A-Za-z0-9._-]+$/.test(name) ||
      !name.endsWith(suffix)
    )
      throw Error(`Invalid update URL for ${target}.`);
    if (
      !Number.isSafeInteger(entry.size) ||
      entry.size <= 0 ||
      entry.size > maximum ||
      !/^[a-f0-9]{64}$/.test(entry.sha256)
    )
      throw Error("Invalid update size or checksum.");
    if (
      typeof entry.signature !== "string" ||
      !entry.signature ||
      entry.signature.length > 8192
    )
      throw Error("Missing update signature.");
  }
  if (JSON.stringify(feed).length > 256 * 1024)
    throw Error("Update feed is too large.");
  return feed;
}
// Tauri uses Minisign's prehashed Ed25519 format. Check both signatures: the
// global signature authenticates the trusted version comment as well as the file.
export function verifyArtifact(bytes, signature, encodedKey, version) {
  const keyLines = Buffer.from(encodedKey, "base64")
    .toString("utf8")
    .trim()
    .split(/\r?\n/);
  const publicBytes = Buffer.from(keyLines[1] || "", "base64");
  const lines = Buffer.from(signature, "base64")
    .toString("utf8")
    .trim()
    .split(/\r?\n/);
  const signed = Buffer.from(lines[1] || "", "base64");
  const global = Buffer.from(lines[3] || "", "base64");
  if (
    publicBytes.length !== 42 ||
    publicBytes.subarray(0, 2).toString() !== "Ed" ||
    signed.length !== 74 ||
    signed.subarray(0, 2).toString() !== "ED" ||
    !publicBytes.subarray(2, 10).equals(signed.subarray(2, 10)) ||
    global.length !== 64 ||
    !lines[2]?.startsWith("trusted comment: ")
  )
    throw Error("Invalid Minisign key or signature.");
  const key = createPublicKey({
    format: "der",
    type: "spki",
    key: Buffer.concat([
      Buffer.from("302a300506032b6570032100", "hex"),
      publicBytes.subarray(10),
    ]),
  });
  const comment = lines[2].slice(17);
  const fileSignature = signed.subarray(10);
  if (
    !verify(
      null,
      createHash("blake2b512").update(bytes).digest(),
      key,
      fileSignature,
    ) ||
    !verify(
      null,
      Buffer.concat([fileSignature, Buffer.from(comment)]),
      key,
      global,
    )
  )
    throw Error("Update signature verification failed.");
  const signedVersion = comment
    .split("\t")
    .find((part) => part.startsWith("version:"))
    ?.slice(8);
  if (signedVersion !== version)
    throw Error("Update signature does not bind the announced version.");
}
export function prepare({
  version,
  notes = "",
  date,
  assets,
  key = publicKey(),
}) {
  if (!stable.test(version) || version === "0.0.0")
    throw Error("Choose a new stable version.");
  const platforms = {};
  for (const [target, file] of assets) {
    if (platforms[target]) throw Error(`Duplicate update target: ${target}`);
    const bytes = readFileSync(file);
    const signature = readFileSync(`${file}.sig`, "utf8").trim();
    verifyArtifact(bytes, signature, key, version);
    platforms[target] = {
      url: `https://github.com/FerMatter/fermatter.github.io/releases/download/v${version}/${path.basename(file)}`,
      signature,
      size: bytes.length,
      sha256: createHash("sha256").update(bytes).digest("hex"),
    };
  }
  return validateFeed({
    version,
    notes,
    pub_date: new Date(date).toISOString(),
    platforms,
  });
}
if (isMain(import.meta.url)) {
  try {
    const args = process.argv.slice(2);
    const options = { assets: [] };
    for (let i = 0; i < args.length; i += 2) {
      const value = args[i + 1];
      if (!value) throw Error(`Missing value for ${args[i]}`);
      if (args[i] === "--asset") {
        const split = value.indexOf("=");
        if (split < 1) throw Error("Use --asset target=/path/to/artifact");
        options.assets.push([value.slice(0, split), value.slice(split + 1)]);
      } else if (args[i] === "--notes-file")
        options.notes = readFileSync(value, "utf8");
      else if (["--version", "--date", "--output"].includes(args[i]))
        options[args[i].slice(2)] = value;
      else throw Error(`Unknown option: ${args[i]}`);
    }
    if (!options.output || !options.date)
      throw Error(
        "Required: --version X.Y.Z --date ISO_DATE --asset TARGET=FILE --output NEW_JSON_FILE [--notes-file FILE]",
      );
    const feed = prepare(options);
    writeFileSync(options.output, JSON.stringify(feed, null, 2) + "\n", {
      flag: "wx",
    });
    console.log(
      `Verified ${Object.keys(feed.platforms).length} signed update artifacts; wrote ${options.output}. Nothing published.`,
    );
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
