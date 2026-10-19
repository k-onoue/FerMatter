// Real CLI/MCP/resident smoke test, with synthetic data and a separate resident identity.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { createHash, randomUUID } from "node:crypto";
import {
  mkdir,
  mkdtemp,
  readFile,
  writeFile,
  realpath,
} from "node:fs/promises";
import { createInterface } from "node:readline";
import { resolve, join } from "node:path";
import { setTimeout as delay } from "node:timers/promises";

const executable = resolve(
  process.argv[2] ||
    "desktop-app/src-tauri/target/release/bundle/macos/Mathlidelux.app/Contents/MacOS/Mathlidelux",
);
const parent = resolve("desktop-app/build");
await mkdir(parent, { recursive: true });
const output = await realpath(await mkdtemp(join(parent, "automation-ipc-")));
const home = join(output, "data"),
  root = join(output, "Automation QA");
await mkdir(root);
const original = "#let projectValue = $x + 7$\n= Model\n$projectValue$\n";
await writeFile(join(root, "main.typ"), original);
await writeFile(
  join(root, ".mathlidelux-project.json"),
  JSON.stringify({
    schemaVersion: 1,
    kind: "mathlidelux.project",
    uuid: randomUUID(),
  }),
);
const id = createHash("sha256").update(root).digest("hex").slice(0, 24);
const projectDirectory = join(home, "Projects", id);
await mkdir(projectDirectory, { recursive: true });
const card = {
  id: "synthetic",
  tex: "projectValue",
  name: "Project value",
  kind: "block",
  origin: "source",
  file: "main.typ",
  line: 3,
  sourceOrder: 0,
  typstContext: [
    { file: "main.typ", offset: original.indexOf("$projectValue$") },
  ],
};
const project = {
  id,
  root,
  main: "main.typ",
  format: "typst",
  engine: "pdflatex",
  cards: [card],
  exportFontSize: 30,
};
await writeFile(
  join(projectDirectory, "project.json"),
  JSON.stringify(project),
);
await writeFile(
  join(home, "Setup.json"),
  JSON.stringify({
    schemaVersion: 1,
    complete: true,
    directory: output,
    demos: join(output, "Demos"),
    token: randomUUID(),
    projects: ["demo-paper", "demo-typst", "demo-tikz", "demo-cetz"].map(
      (name) => ({ name, source: null }),
    ),
  }),
);
await mkdir(join(home, "Settings"), { recursive: true });
// Optional compiler path makes version compatibility checks reproducible without
// replacing the user's installed Typst or saved application settings.
if (process.argv[3]) {
  await writeFile(
    join(home, "Settings", "tools.json"),
    JSON.stringify({
      schemaVersion: 1,
      kind: "mathlidelux.tools",
      paths: { typst: await realpath(process.argv[3]) },
    }),
  );
}
const emptyKey = {
  key: "",
  command: false,
  control: false,
  option: false,
  shift: false,
};
await writeFile(
  join(home, "Settings", "keybindings.json"),
  JSON.stringify({
    schemaVersion: 1,
    kind: "mathlidelux.keybindings",
    bindings: { globalInstant: emptyKey, globalInstantClipboard: emptyKey },
  }),
);
const env = { ...process.env, MATHLIDELUX_DATA_DIR: home };
const processes = [];
const launch = (args) => {
  const child = spawn(executable, args, {
    env,
    stdio: ["pipe", "pipe", "pipe"],
  });
  processes.push(child);
  return child;
};
const resident = launch(["--automation-background"]);
let residentErrors = "";
resident.stderr.on("data", (b) => (residentErrors += b));
resident.stdout.resume();
async function cli(tool, arguments_ = {}) {
  const child = launch(["cli", tool, "--input", "-"]);
  let stdout = "",
    stderr = "";
  child.stdout.on("data", (b) => (stdout += b));
  child.stderr.on("data", (b) => (stderr += b));
  child.stdin.end(JSON.stringify(arguments_));
  const code = await new Promise((done, fail) => {
    child.on("exit", done);
    child.on("error", fail);
  });
  const result = JSON.parse(stdout);
  return { code, ...result, stderr };
}
try {
  let listing;
  for (let attempt = 0; attempt < 100; attempt++) {
    await delay(100);
    listing = await cli("list_projects");
    if (listing.result) break;
    if (resident.exitCode !== null)
      throw new Error(`Resident exited: ${residentErrors}`);
  }
  assert.equal(listing.result?.projects[0]?.id, id, JSON.stringify(listing));
  const templates = await cli("list_templates");
  assert(templates.result.templates.some((t) => t.id === "builtin-cetz"));
  const rendering = await cli("render_equation", {
    projectId: id,
    source: "projectValue + 1",
    fontSize: 30,
    outputDirectory: output,
  });
  assert.equal(rendering.code, 0, JSON.stringify(rendering));
  const item = rendering.result.items[0];
  assert.equal(item.status, "ready");
  assert(item.widthPt > 0 && item.heightPt > 0);
  assert((await readFile(item.path, "utf8")).includes("mathlidelux"));
  assert((await readFile(item.previewPath)).length > 0);
  const search = await cli("search_equations", {
    projectId: id,
    query: "projectValue",
  });
  const variantArgs = {
    projectId: id,
    equationId: card.id,
    source: "projectValue + 2",
    name: "API variant",
    expectedRevision: search.result.revision,
    requestId: randomUUID(),
  };
  const variant = await cli("create_variant", variantArgs);
  assert.equal(variant.result?.created, true, JSON.stringify(variant));
  assert.equal(
    (await cli("create_variant", variantArgs)).result.created,
    false,
  );
  const conflict = await cli("create_variant", {
    ...variantArgs,
    requestId: randomUUID(),
  });
  assert.equal(conflict.error.code, "project_conflict");
  assert.equal(conflict.code, 1);
  const invalid = await cli("render_equation", {
    source: "x",
    format: "gif",
    outputDirectory: output,
  });
  assert.equal(invalid.error.code, "invalid_arguments");

  const mcp = launch(["mcp"]);
  const waiting = new Map();
  let sequence = 0;
  const notifications = [];
  const lines = createInterface({ input: mcp.stdout });
  let protocolError;
  lines.on("line", (line) => {
    try {
      const message = JSON.parse(line);
      if ("id" in message) {
        waiting.get(message.id)?.(message);
        waiting.delete(message.id);
      } else {
        notifications.push(message);
      }
    } catch (error) {
      protocolError = error;
    }
  });
  mcp.stderr.resume();
  const send = (message) =>
    mcp.stdin.write(JSON.stringify({ jsonrpc: "2.0", ...message }) + "\n");
  const request = (method, params) => {
    const id = ++sequence;
    const promise = new Promise((done, fail) => {
      const timeout = setTimeout(() => {
        waiting.delete(id);
        fail(new Error(`MCP timeout: ${method}`));
      }, 60000);
      timeout.unref();
      waiting.set(id, (message) => {
        clearTimeout(timeout);
        done(message);
      });
    });
    send({ id, method, params });
    return promise;
  };
  const initialized = await request("initialize", {
    protocolVersion: "2025-11-25",
    capabilities: {},
    clientInfo: { name: "synthetic-verifier", version: "1" },
  });
  assert(initialized.result.capabilities.tools);
  send({ method: "notifications/initialized" });
  const available = await request("tools/list", {});
  assert.equal(available.result.tools.length, 8);
  const projects = await request("tools/call", {
    name: "list_projects",
    arguments: {},
  });
  assert.equal(projects.result.structuredContent.projects[0].id, id);
  const exportResult = await request("tools/call", {
    name: "export_equations",
    arguments: {
      projectId: id,
      equationIds: [variant.result.equation.id, card.id],
      format: "svg",
      fontSize: 30,
      outputDirectory: output,
    },
    _meta: { progressToken: "export-check" },
  });
  assert.equal(exportResult.result.isError, false);
  assert.deepEqual(
    exportResult.result.structuredContent.items.map((i) => i.equationId),
    [variant.result.equation.id, card.id],
  );
  assert(notifications.some((n) => n.method === "notifications/progress"));
  const bad = await request("tools/call", {
    name: "render_equation",
    arguments: { source: "x", fontSize: -1, outputDirectory: output },
  });
  assert.equal(bad.result.isError, true);
  assert.equal(bad.result.structuredContent.error.code, "invalid_arguments");
  // TeX's infinite loop is intentionally synthetic. Cancellation must free its compiler slot.
  const pending = request("tools/call", {
    name: "render_equation",
    arguments: {
      language: "latex",
      kind: "figure",
      source: "\\loop\\iftrue\\repeat",
      outputDirectory: output,
    },
  });
  const cancellationId = sequence;
  await delay(500);
  send({
    method: "notifications/cancelled",
    params: { requestId: cancellationId, reason: "verification" },
  });
  // MCP permits servers to omit a response for a cancelled request.
  const cancelled = await Promise.race([pending, delay(1500, null)]);
  if (cancelled)
    assert(
      cancelled.result?.structuredContent?.error?.code === "cancelled" ||
        cancelled.error?.code === -32800,
    );
  waiting.delete(cancellationId);
  const after = await request("tools/call", {
    name: "render_equation",
    arguments: { language: "typst", source: "x+1", outputDirectory: output },
  });
  assert.equal(after.result.isError, false);
  assert.equal(await readFile(join(root, "main.typ"), "utf8"), original);
  assert.equal(
    protocolError,
    undefined,
    "MCP stdout must contain only JSON-RPC",
  );
  // Explicit opt-in for a visible review handoff; never opens the user's projects.
  if (process.env.MATHLIDELUX_VERIFY_OPEN === "1")
    assert.equal(
      (await cli("open_in_app", { path: item.path })).result.status,
      "queued",
    );
  await writeFile(
    join(output, "verification.json"),
    JSON.stringify(
      {
        cli: true,
        mcp: true,
        cancellation: true,
        sourcePreserved: true,
        openQueued: process.env.MATHLIDELUX_VERIFY_OPEN === "1",
        render: rendering.result,
      },
      null,
      2,
    ),
  );
  console.log(
    `PASS: isolated CLI/MCP, project definitions, SVG/preview/dimensions, variant retry/conflict, ordered batch export, progress, cancellation.\n${output}`,
  );
  if (process.env.MATHLIDELUX_VERIFY_OPEN === "1")
    await delay(
      Math.min(60000, Number(process.env.MATHLIDELUX_VERIFY_HOLD_MS || 0)),
    );
} finally {
  for (const child of processes)
    if (child.exitCode === null) child.kill("SIGTERM");
  await delay(300);
  for (const child of processes)
    if (child.exitCode === null) child.kill("SIGKILL");
}
