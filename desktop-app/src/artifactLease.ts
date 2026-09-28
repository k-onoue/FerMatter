import { startDrag as nativeDrag } from "@crabnebula/tauri-plugin-drag";
import type { Options } from "@crabnebula/tauri-plugin-drag";
import { rpc, uid } from "./api";
import { useEffect } from "react";
import type { Card } from "./types";

export function retainArtifacts(paths: string[]) {
  const key = `artifacts::${uid()}`;
  const acquired = rpc("artifact_lease", { key, paths });
  let released = false;
  return {
    acquired,
    async release() {
      if (released) return;
      released = true;
      // A quick unmount must not enqueue release ahead of acquisition.
      await acquired;
      await rpc("artifact_lease", { key, paths: [] });
    },
  };
}

export async function startDrag(options: Options) {
  const lease = retainArtifacts([
    ...(Array.isArray(options.item) ? options.item : []),
    options.icon,
  ]);
  const release = () => {
    void lease.release().catch(console.error);
  };
  try {
    await lease.acquired;
    // The invoke promise can resolve before the native drag ends. Only its
    // Dropped/Cancelled callback releases files needed by the receiving app.
    await nativeDrag(options, release);
  } catch (error) {
    release();
    throw error;
  }
}

export function useCardArtifacts(
  card: Card | null | undefined,
  extra?: string,
) {
  const svg = card?.svgPath,
    png = card?.pngPath,
    pdf = card?.pdfPath;
  useEffect(() => {
    const paths = [svg, png, pdf, extra].filter(
      (path): path is string => !!path,
    );
    if (!paths.length) return;
    const lease = retainArtifacts(paths);
    void lease.acquired.catch(console.error);
    return () => {
      void lease.release().catch(console.error);
    };
  }, [svg, png, pdf, extra]);
}
