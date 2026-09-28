/** Model lifetime is a project session, with independent references for split editors. */
export class ModelOwnership<T> {
  private entries = new Map<
    string,
    { project: string; value: T; references: number; retain: boolean }
  >();
  constructor(private readonly dispose: (value: T, key: string) => void) {}
  acquire(project: string, key: string, value: T, retain: boolean) {
    const entry = this.entries.get(key) || {
      project,
      value,
      references: 0,
      retain: false,
    };
    entry.references++;
    entry.retain ||= retain;
    this.entries.set(key, entry);
    return () => {
      entry.references--;
      this.collect(key, entry);
    };
  }
  releaseProject(project: string) {
    for (const [key, entry] of this.entries)
      if (entry.project === project) {
        entry.retain = false;
        this.collect(key, entry);
      }
  }
  private collect(
    key: string,
    entry: { value: T; references: number; retain: boolean },
  ) {
    if (!entry.references && !entry.retain) {
      this.entries.delete(key);
      this.dispose(entry.value, key);
    }
  }
  get size() {
    return this.entries.size;
  }
}
