import type { Buffer } from "./types";

export function dirtySourceBuffers(buffers: Record<string, Buffer>) {
  return Object.fromEntries(
    Object.entries(buffers).filter(([, buffer]) => buffer.dirty),
  );
}

/** A close/switch may arrive before the debounce timer. Drafts are the safety boundary. */
export async function flushSourceBuffers(options: {
  pending: () => Promise<void>[];
  buffers: () => Record<string, Buffer>;
  saveDrafts: (value: Record<string, Buffer>) => Promise<unknown>;
  save: (path: string) => Promise<unknown>;
  autoSave: boolean;
}) {
  await Promise.allSettled(options.pending());
  await options.saveDrafts(dirtySourceBuffers(options.buffers()));
  if (options.autoSave) {
    await Promise.allSettled(
      Object.values(options.buffers())
        .filter((buffer) => buffer.dirty && !buffer.recovered && !buffer.error)
        .map((buffer) => options.save(buffer.path)),
    );
  }
  let snapshot;
  do {
    snapshot = options.buffers();
    await options.saveDrafts(dirtySourceBuffers(snapshot));
  } while (snapshot !== options.buffers());
}
