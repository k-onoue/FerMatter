import {
  cpSync,
  mkdirSync,
  existsSync,
  readFileSync,
  writeFileSync,
  readdirSync,
} from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const target = join(root, "public/vendor/pdfjs");
mkdirSync(target, { recursive: true });
for (const folder of ["cmaps", "standard_fonts", "wasm"]) {
  const source = join(root, "node_modules/pdfjs-dist", folder);
  if (existsSync(source))
    cpSync(source, join(target, folder), { recursive: true });
}
// Ship the exact license texts belonging to the production npm dependency graph.
const lock = JSON.parse(readFileSync(join(root, "package-lock.json"), "utf8"));
let notices = "# JavaScript dependencies\n\n";
for (const [path, meta] of Object.entries(lock.packages)) {
  if (!path || meta.dev || !existsSync(join(root, path))) continue;
  const packageJson = join(root, path, "package.json");
  if (!existsSync(packageJson)) continue;
  const pkg = JSON.parse(readFileSync(packageJson, "utf8"));
  notices += `\n## ${pkg.name} ${pkg.version}\n\nLicense: ${pkg.license || "See bundled license"}\n\n`;
  for (const name of readdirSync(join(root, path)).filter((n) =>
    /^(licen[sc]e|notice|copyright)/i.test(n),
  )) {
    try {
      notices += readFileSync(join(root, path, name), "utf8") + "\n";
    } catch {}
  }
}
// These monorepo packages omit their repository-root license from npm/crates.
// Keep the release-matched originals locally so offline builds ship the texts.
notices += "\n## unified-latex packages (repository license)\n\n" +
  readFileSync(join(root, "resources/formatters/unified-latex-LICENSE"), "utf8");
writeFileSync(join(root, "public/vendor/JAVASCRIPT-LICENSES.txt"), notices);
// Cargo resolves platform-specific runtime/build dependencies; retain their source notices.
const { execFileSync } = await import("node:child_process");
const env = { ...process.env };
if (existsSync(join(root, "build/rustup/toolchains")))
  env.RUSTUP_HOME = join(root, "build/rustup");
const targetTriple =
  process.platform === "darwin"
    ? `${process.arch === "arm64" ? "aarch64" : "x86_64"}-apple-darwin`
    : undefined;
const metadata = JSON.parse(
  execFileSync(
    "cargo",
    [
      "metadata",
      "--manifest-path",
      join(root, "src-tauri/Cargo.toml"),
      "--format-version",
      "1",
      "--locked",
      ...(targetTriple ? ["--filter-platform", targetTriple] : []),
    ],
    { env, encoding: "utf8", maxBuffer: 32 * 1024 * 1024 },
  ),
);
const included = new Set(metadata.resolve.nodes.map((n) => n.id));
let rustNotices = "# Rust dependencies\n\n";
for (const pkg of metadata.packages.filter(
  (p) => included.has(p.id) && p.source,
)) {
  const directory = dirname(pkg.manifest_path);
  rustNotices += `\n## ${pkg.name} ${pkg.version}\n\nLicense: ${pkg.license || "See bundled license"}\nSource: ${pkg.repository || pkg.source}\n\n`;
  const files = new Set(
    readdirSync(directory).filter((n) =>
      /^(licen[sc]e|notice|copyright)/i.test(n),
    ),
  );
  if (pkg.license_file) files.add(pkg.license_file);
  for (const file of files) {
    try {
      rustNotices += readFileSync(join(directory, file), "utf8") + "\n";
    } catch {}
  }
}
for (const name of ["typstyle", "typst"])
  rustNotices += `\n## ${name} crates (repository license)\n\n` +
    readFileSync(join(root, `resources/formatters/${name}-LICENSE`), "utf8");
writeFileSync(join(root, "public/vendor/RUST-LICENSES.txt"), rustNotices);
