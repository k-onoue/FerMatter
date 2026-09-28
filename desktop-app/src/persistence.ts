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

const handlers = new Map<string, () => Promise<void>>();
export function registerWindowFlush(key: string, flush: () => Promise<void>) {
  handlers.set(key, flush);
  return () => {
    if (handlers.get(key) === flush) handlers.delete(key);
  };
}
export async function flushWindow() {
  // A source failure must not prevent another editor from preserving its draft.
  const results = await Promise.allSettled(
    [...handlers.values()].map((flush) => flush()),
  );
  const errors = results.flatMap((result) =>
    result.status === "rejected" ? [String(result.reason)] : [],
  );
  if (errors.length) throw new Error(errors.join("\n"));
}
