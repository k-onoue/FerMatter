import type { KeyboardEvent as ReactKeyboardEvent } from "react";
import type { Binding, Keybindings } from "./types";
import { isWindows } from "./platform";

const shortcut = (key: string, options: Partial<Binding> = {}): Binding => ({
  key,
  command: true,
  control: false,
  shift: false,
  option: false,
  ...options,
});
export const commands: { id: string; name: string; binding: Binding }[] = [
  ["open", "Open Project", shortcut("o")],
  ["projects", "Projects", shortcut("o", { shift: true })],
  ["newProject", "New Project", shortcut("n", { shift: true })],
  [
    "globalInstant",
    "Instant from Anywhere",
    shortcut("n", { control: true, option: true }),
  ],
  [
    "globalInstantClipboard",
    "Edit SVG from Clipboard",
    shortcut("v", { control: true, option: true }),
  ],
  ["instant", "New Instant", shortcut("k", { shift: true })],
  ["newEquation", "New Equation", shortcut("n")],
  ["newFigure", "New Figure", shortcut("n", { option: true })],
  ["find", "Find", shortcut("f")],
  ["edit", "Edit Selection", shortcut("e")],
  ["duplicate", "Duplicate Selection", shortcut("d")],
  ["saveEquation", "Save", shortcut("s")],
  ["copySVG", "Copy Image", shortcut("c", { shift: true })],
  ["exportSVG", "Export Image", shortcut("s", { shift: true })],
  ["rescan", "Rescan", shortcut("r")],
  ["togglePDF", "Toggle PDF", shortcut("p", { option: true })],
  ["rebuildPDF", "Rebuild PDF", shortcut("r", { shift: true })],
  ["previousPage", "Previous PDF Page", shortcut("[")],
  ["nextPage", "Next PDF Page", shortcut("]")],
  ["zoomIn", "Zoom PDF In", shortcut("=", { control: true })],
  ["zoomOut", "Zoom PDF Out", shortcut("-", { control: true })],
  ["fitPDF", "Fit PDF", shortcut("0", { control: true })],
  ["zoomWindowIn", "Zoom In", shortcut("=")],
  ["zoomWindowOut", "Zoom Out", shortcut("-")],
  ["resetWindowZoom", "Actual Size", shortcut("0")],
  ["inspector", "Toggle Inspector", shortcut("i", { option: true })],
  ["pin", "Always on Top", shortcut("p", { shift: true })],
  ["fullScreen", "Full Screen", shortcut("f", { control: true })],
  ["projectSource", "Toggle Source", shortcut("e", { option: true })],
  [
    "nextSourceTab",
    "Next Source Tab",
    shortcut("\t", { command: isWindows(), control: !isWindows() }),
  ],
  [
    "previousSourceTab",
    "Previous Source Tab",
    shortcut("\t", {
      command: isWindows(),
      control: !isWindows(),
      shift: true,
    }),
  ],
  ["splitSource", "Split Editor Right", shortcut("\\")],
  ["splitSourceDown", "Split Editor Down", shortcut("\\", { option: true })],
].map(([id, name, binding]) => ({
  id: id as string,
  name: name as string,
  binding: binding as Binding,
}));
export function eventBinding(e: KeyboardEvent | ReactKeyboardEvent): Binding {
  const special: Record<string, string> = {
    Tab: "\t",
    ArrowUp: "\uF700",
    ArrowDown: "\uF701",
    ArrowLeft: "\uF702",
    ArrowRight: "\uF703",
  };
  let key = special[e.key] || e.key.toLowerCase();
  const shifted = '~!@#$%^&*()_+{}|:"<>?',
    plain = "`1234567890-=[]\\;',./";
  if (e.shiftKey && shifted.includes(key)) key = plain[shifted.indexOf(key)];
  if (key === "+") key = "="; // Numeric keypad and layouts with an unshifted +.
  return {
    key,
    command: isWindows() ? e.ctrlKey : e.metaKey,
    control: isWindows() ? e.metaKey : e.ctrlKey,
    shift: e.shiftKey,
    option: e.altKey,
  };
}
export const bindingEqual = (a: Binding, b: Binding) =>
  a.key === b.key &&
  a.command === b.command &&
  a.control === b.control &&
  a.shift === b.shift &&
  a.option === b.option;
const compatibleDefaults = [
  "globalInstant",
  "globalInstantClipboard",
  "instant",
  "zoomIn",
  "zoomOut",
  "fitPDF",
  "zoomWindowIn",
  "zoomWindowOut",
  "resetWindowZoom",
];
export function effectiveBindings(keys: Keybindings): Record<string, Binding> {
  const bindings = Object.fromEntries(
    commands.map((c) => [c.id, keys.bindings[c.id] || c.binding]),
  );
  // New defaults yield to saved overrides, including legacy PDF shortcuts.
  // Explicit duplicate assignments still fail validation.
  return Object.fromEntries(
    Object.entries(bindings).map(([id, binding]) => [
      id,
      compatibleDefaults.includes(id) &&
      !keys.bindings[id] &&
      Object.entries(keys.bindings).some(
        ([other, saved]) => other !== id && bindingEqual(saved, binding),
      )
        ? { ...binding, key: "" }
        : binding,
    ]),
  );
}
export function commandForEvent(event: KeyboardEvent, keys: Keybindings) {
  if (event.isComposing) return;
  const binding = eventBinding(event);
  const bindings = effectiveBindings(keys);
  const exact = commands.find((c) => bindingEqual(bindings[c.id], binding));
  if (exact) return exact.id;
  // WebKit reports the unshifted key with Command held: JIS + arrives as
  // Shift+Semicolon, and JIS = as Shift+Minus. Explicit assignments win above.
  const jisZoomIn = binding.shift && [";", "-"].includes(binding.key);
  if (
    (binding.key === "=" && (event.key === "+" || binding.shift)) ||
    jisZoomIn
  ) {
    return ["zoomWindowIn", "zoomIn"].find((id) =>
      bindingEqual(bindings[id], { ...binding, key: "=", shift: false }),
    );
  }
}
export function validateBindings(value: Keybindings) {
  if (
    value.schemaVersion !== 1 ||
    value.kind !== "mathlidelux.keybindings" ||
    !value.bindings
  )
    throw new Error("Invalid keyboard shortcut file.");
  const seen: Binding[] = [];
  const bindings = effectiveBindings(value);
  for (const id of Object.keys(value.bindings))
    if (!commands.some((c) => c.id === id))
      throw new Error(`Unknown command: ${id}`);
  for (const command of commands) {
    const b = bindings[command.id];
    if (
      ["command", "control", "shift", "option"].some(
        (k) => typeof b[k as keyof Binding] !== "boolean",
      )
    )
      throw new Error("Shortcut modifiers must be true or false.");
    if (!b.key) continue;
    if (b.key.length !== 1 || (!b.command && !b.control))
      throw new Error("Use Command or Control with one key.");
    if (
      (b.command &&
        !b.control &&
        !b.option &&
        !b.shift &&
        (isWindows() ? "qwmh,cvxazp " : "qwmh,cvxazp \t").includes(b.key)) ||
      (b.command && b.shift && !b.control && !b.option && b.key === "z")
    )
      throw new Error("Reserved for the system or text editing.");
    if (seen.some((a) => bindingEqual(a, b)))
      throw new Error(`Duplicate shortcut: ${command.name}`);
    seen.push(b);
  }
}
export function bindingKeys(b: Binding) {
  return [
    ...(b.control ? [isWindows() ? "Win" : "⌃"] : []),
    ...(b.option ? [isWindows() ? "Alt" : "⌥"] : []),
    ...(b.shift ? [isWindows() ? "Shift" : "⇧"] : []),
    ...(b.command ? [isWindows() ? "Ctrl" : "⌘"] : []),
    b.key === "\t" ? "⇥" : b.key.toUpperCase(),
  ].filter(Boolean);
}
