export type SettingsSnapshot<T> = { value: T; revision: string };

/** One document per caller. Mutations are optimistic; responses only advance revisions. */
export class SettingsWriter<T extends object> {
  private value: T;
  private revision = "";
  private tail: Promise<void> = Promise.resolve();
  private failure: unknown;
  private reloading = false;
  constructor(
    initial: T,
    private readonly write: (value: T, revision: string) => Promise<string>,
    private readonly display: (value: T) => void,
    private readonly optimistic = true,
  ) {
    this.value = initial;
  }
  accept(snapshot: SettingsSnapshot<T>) {
    this.value = snapshot.value;
    this.revision = snapshot.revision;
    this.failure = undefined;
    this.display(this.value);
  }
  patch(patch: Partial<T>) {
    return this.replace({ ...this.value, ...patch });
  }
  replace(value: T) {
    if (this.reloading)
      return Promise.reject(
        new Error("Settings are reloading. Try again after Reload completes."),
      );
    if (this.failure) return Promise.reject(this.failure);
    this.value = value;
    if (this.optimistic) this.display(value);
    const next = this.tail.then(async () => {
      if (this.failure) throw this.failure;
      try {
        this.revision = await this.write(value, this.revision);
        if (!this.optimistic && this.value === value) this.display(value);
      } catch (error) {
        this.failure = error;
        throw error;
      }
    });
    this.tail = next.catch(() => {});
    return next;
  }
  async flush() {
    await this.tail;
    if (this.failure) throw this.failure;
  }
  async reload(read: () => Promise<SettingsSnapshot<T>>) {
    if (this.reloading) return;
    this.reloading = true;
    try {
      await this.tail;
      this.accept(await read());
    } finally {
      this.reloading = false;
    }
  }
}
