import type { DragEvent } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { rpc } from "./api";

export async function openSVG() {
  const path = await open({
    multiple: false,
    filters: [{ name: "SVG", extensions: ["svg"] }],
  });
  if (typeof path === "string") await rpc("import_svg", { path });
}

export function dropSVG(event: DragEvent, report: (error: unknown) => void) {
  const files = Array.from(event.dataTransfer.files);
  if (
    !files.length ||
    !files.every((file) => file.name.toLowerCase().endsWith(".svg"))
  )
    return false;
  event.preventDefault();
  event.stopPropagation();
  void (async () => {
    if (files.length > 16 || files.some((file) => file.size > 16 * 1024 * 1024))
      throw new Error("Drop up to 16 SVG files, each smaller than 16 MB.");
    for (const file of files)
      await rpc("import_svg", { svg: await file.text() });
  })().catch(report);
  return true;
}
