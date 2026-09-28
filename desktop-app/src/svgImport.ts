import type { DragEvent } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { rpc } from "./api";
import { acceptsSVG, readSVGTransfer } from "./svgTransfer";

export async function openSVG(target?: "workspace") {
  const path = await open({
    multiple: false,
    filters: [{ name: "SVG", extensions: ["svg"] }],
  });
  if (typeof path === "string") await rpc("import_svg", { path, target });
}

export function dropSVG(
  event: DragEvent,
  report: (error: unknown) => void,
  target?: "workspace",
) {
  // The drag data store becomes protected when the synchronous event ends.
  const transfer = {
    files: Array.from(event.dataTransfer.files),
    svg: event.dataTransfer.getData("image/svg+xml"),
    html: event.dataTransfer.getData("text/html"),
    text: event.dataTransfer.getData("text/plain"),
  };
  if (!acceptsSVG(transfer)) return false;
  event.preventDefault();
  event.stopPropagation();
  void (async () => {
    const sources = await readSVGTransfer(transfer);
    await rpc("import_svg", { sources, target });
  })().catch(report);
  return true;
}
