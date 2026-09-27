import { invoke } from "@tauri-apps/api/core";
export function rpc<T = unknown>(
  operation: string,
  payload: Record<string, unknown> = {},
): Promise<T> {
  return invoke("request", { operation, payload });
}
const assets = new Map<string, Promise<string>>();
export function asset(path: string) {
  let value = assets.get(path);
  if (!value) {
    value = rpc<string>("artifact", { path });
    assets.set(path, value);
    value.catch(() => assets.delete(path));
    if (assets.size > 256) assets.delete(assets.keys().next().value!);
  }
  return value;
}
export function projectName(root: string) {
  return root.split("/").filter(Boolean).at(-1) || "Project";
}
export const uid = () => crypto.randomUUID();
