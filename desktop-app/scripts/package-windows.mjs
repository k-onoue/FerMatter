import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  writeFileSync,
  constants,
} from "node:fs";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { assertEvidence } from "./release-report.mjs";

const desktop = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repository = dirname(desktop);
const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");

export function peMachine(bytes) {
  if (bytes.length < 64 || bytes.toString("ascii", 0, 2) !== "MZ")
    throw new Error("Not a Windows executable (missing DOS header).");
  const offset = bytes.readUInt32LE(60);
  if (
    offset < 64 ||
    offset > bytes.length - 24 ||
    bytes.toString("ascii", offset, offset + 4) !== "PE\0\0"
  )
    throw new Error("Invalid or truncated Windows PE header.");
  return bytes.readUInt16LE(offset + 4);
}

export function nsisApplicationBytes(bytes) {
  // Tauri patches this three-byte bundle marker while building the installer,
  // then restores the loose executable. Account for that exact change only.
  const marker = Buffer.from("__TAURI_BUNDLE_TYPE_VAR_");
  const offset = bytes.indexOf(marker);
  if (offset < 0 || bytes.indexOf(marker, offset + 1) >= 0)
    throw new Error("Expected exactly one Tauri bundle type marker.");
  const typeOffset = offset + marker.length;
  const bundleType = bytes.toString("ascii", typeOffset, typeOffset + 3);
  if (!["UNK", "NSS"].includes(bundleType))
    throw new Error(`Unexpected Tauri bundle type: ${bundleType}`);
  const bundled = Buffer.from(bytes);
  bundled.write("NSS", typeOffset, "ascii");
  return bundled;
}

export function preserveFile(path, bytes) {
  if (existsSync(path)) {
    if (!readFileSync(path).equals(bytes))
      throw new Error(`Refusing to replace an existing artifact: ${path}`);
  } else {
    writeFileSync(path, bytes, { flag: "wx" });
  }
}

function command(executable, args) {
  return execFileSync(executable, args, {
    cwd: repository,
    encoding: "utf8",
    windowsHide: true,
  }).trim();
}

export function packageWindows(
  outputDirectory,
  channel = "prototype",
  validation,
) {
  if (process.platform !== "win32") throw new Error("Package on Windows.");
  if (!["prototype", "preview", "stable-candidate"].includes(channel))
    throw new Error("Choose prototype, preview or stable-candidate.");
  const tauri = JSON.parse(
    readFileSync(join(desktop, "src-tauri/tauri.conf.json"), "utf8"),
  );
  const metadata = JSON.parse(
    readFileSync(join(desktop, "packaging/windows.json"), "utf8"),
  );
  if (
    metadata.architecture !== "x64" ||
    metadata.target !== "x86_64-pc-windows-msvc" ||
    !Number.isSafeInteger(metadata.preview) ||
    metadata.preview < 1
  )
    throw new Error("Unsupported Windows package metadata.");
  const version = tauri.version;
  if (!/^\d+\.\d+\.\d+$/.test(version))
    throw new Error("Expected a numeric application version.");
  const executable = join(desktop, "src-tauri/target/release/FerMatter.exe");
  const executableBytes = readFileSync(executable);
  if (peMachine(executableBytes) !== 0x8664)
    throw new Error("Expected an x64 Windows application.");
  const bundle = join(desktop, "src-tauri/target/release/bundle/nsis");
  const installers = readdirSync(bundle).filter(
    (name) => name === `FerMatter_${version}_x64-setup.exe`,
  );
  if (installers.length !== 1)
    throw new Error("Expected exactly one matching Tauri NSIS installer.");
  const source = join(bundle, installers[0]);
  const installerBytes = readFileSync(source);
  if (![0x14c, 0x8664].includes(peMachine(installerBytes)))
    throw new Error("Unexpected installer architecture.");
  const commit = command("git", ["rev-parse", "HEAD"]);
  const dirty = command("git", ["status", "--porcelain"]).length > 0;
  if (channel !== "prototype") {
    if (!validation || dirty)
      throw new Error(
        "Distribution candidates require validation and a clean checkout.",
      );
    const report = JSON.parse(readFileSync(validation, "utf8"));
    assertEvidence(report, commit, bundle);
    if (report.version !== version)
      throw new Error("Validation version differs from app.");
  }
  const output = resolve(outputDirectory || join(desktop, "build/windows"));
  mkdirSync(output, { recursive: true });
  const label = channel === "preview" ? `preview.${metadata.preview}` : channel;
  const name = `FerMatter_${version}_windows_x64_${label}_unsigned_setup.exe`;
  const target = join(output, name);
  if (existsSync(target)) {
    preserveFile(target, installerBytes);
  } else {
    copyFileSync(source, target, constants.COPYFILE_EXCL);
  }
  const checksum = sha256(installerBytes);
  preserveFile(
    join(output, "SHA256SUMS.txt"),
    Buffer.from(`${checksum}  ${name}\n`),
  );
  const result = {
    version,
    ...metadata,
    identifier: tauri.identifier,
    channel,
    commit,
    dirty,
    file: name,
    sha256: checksum,
    applicationSha256: sha256(nsisApplicationBytes(executableBytes)),
    unbundledApplicationSha256: sha256(executableBytes),
    signing: "unsigned",
    webview: "offlineInstaller",
    compilerTools: "external",
  };
  preserveFile(
    join(output, "build-info.json"),
    Buffer.from(JSON.stringify(result, null, 2) + "\n"),
  );
  console.log(JSON.stringify({ file: target, sha256: checksum }, null, 2));
  if (validation)
    preserveFile(join(output, "validation.json"), readFileSync(validation));
  return result;
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
)
  packageWindows(process.argv[2], process.argv[3], process.argv[4]);
