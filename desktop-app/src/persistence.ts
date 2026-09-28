/** Serializes snapshots without poisoning subsequent saves after one failure. */
export class SnapshotWriter<T> {
  private tail: Promise<void> = Promise.resolve();
  private latest: T | undefined;
  private dirty = false;
  constructor(private readonly write: (value: T) => Promise<unknown>) {}
  save(value: T): Promise<void> {
    this.latest = value;
    this.dirty = true;
    const operation = this.tail
      .catch(() => {})
      .then(async () => {
        await this.write(value);
        if (this.latest === value) this.dirty = false;
      });
    this.tail = operation;
    // The caller receives the original rejection; abandoned UI promises still have a handler.
    void operation.catch(() => {});
    return operation;
  }
  async flush(): Promise<void> {
    await this.tail.catch(() => {});
    while (this.dirty && this.latest !== undefined) {
      await this.save(this.latest);
      await this.tail;
    }
  }
}

let interactionLocks = 0;
const interactiveCallbacks = new Set<() => void>();

/** Defers focus until every save/transition lease is released. Returns cancellation. */
export function runWhenWindowInteractive(callback: () => void) {
  let active = true;
  const run = () => {
    if (!active) return;
    active = false;
    interactiveCallbacks.delete(run);
    callback();
  };
  if (interactionLocks === 0) run();
  else interactiveCallbacks.add(run);
  return () => {
    active = false;
    interactiveCallbacks.delete(run);
  };
}

export function lockWindowInteraction() {
  if (typeof document === "undefined") return () => {};
  interactionLocks++;
  document.documentElement.inert = true;
  let released = false;
  return () => {
    if (released) return;
    released = true;
    interactionLocks--;
    document.documentElement.inert = interactionLocks > 0;
    while (interactionLocks === 0 && interactiveCallbacks.size > 0) {
      const callback = interactiveCallbacks.values().next().value!;
      try {
        callback();
      } catch (error) {
        // A focus callback must not turn a successful document save into a failure.
        console.error(error);
      }
    }
  };
}

const handlers = new Map<string, () => Promise<void>>();
export function registerWindowFlush(key: string, flush: () => Promise<void>) {
  handlers.set(key, flush);
  return () => {
    if (handlers.get(key) === flush) handlers.delete(key);
  };
}
export async function flushWindow(skipKeys: readonly string[] = []) {
  const release = lockWindowInteraction();
  try {
    // A source failure must not prevent another editor from preserving its draft.
    const results = await Promise.allSettled(
      [...handlers.entries()]
        .filter(([key]) => !skipKeys.includes(key))
        .map(([, flush]) => flush()),
    );
    const errors = results.flatMap((result) =>
      result.status === "rejected" ? [String(result.reason)] : [],
    );
    if (errors.length) throw new Error(errors.join("\n"));
  } finally {
    release();
  }
}
