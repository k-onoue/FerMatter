import { describe, expect, it } from "vitest";
import { SettingsWriter } from "./settingsPersistence";

describe("settings caller revisions", () => {
  it("serializes rapid changes without rolling back the latest UI or losing fields", async () => {
    const values: { opacity: number; autoSave: boolean }[] = [];
    const requests: { revision: string; value: (typeof values)[number] }[] = [];
    let complete!: (value: string) => void;
    const writer = new SettingsWriter(
      { opacity: 0.8, autoSave: true },
      async (value, revision) => {
        requests.push({ value, revision });
        if (requests.length === 1)
          return new Promise<string>((resolve) => {
            complete = resolve;
          });
        return "r3";
      },
      (value) => values.push(value),
    );
    writer.accept({ value: { opacity: 0.8, autoSave: true }, revision: "r1" });
    const first = writer.patch({ opacity: 0.9 });
    const second = writer.patch({ autoSave: false });
    const last = writer.patch({ opacity: 1 });
    await Promise.resolve();
    expect(requests).toHaveLength(1);
    expect(values.at(-1)).toEqual({ opacity: 1, autoSave: false });
    complete("r2");
    await Promise.all([first, second, last]);
    expect(requests[1].revision).toBe("r2");
    expect(requests.at(-1)?.value).toEqual({ opacity: 1, autoSave: false });
    expect(values.at(-1)).toEqual({ opacity: 1, autoSave: false });
  });
  it("blocks pending saves after conflict until an explicit successful reload", async () => {
    let calls = 0;
    const writer = new SettingsWriter(
      { opacity: 0.8 },
      async () => {
        calls++;
        if (calls === 1) throw new Error("Reload the file before saving");
        return "accepted";
      },
      () => {},
    );
    writer.accept({ value: { opacity: 0.8 }, revision: "old" });
    const saves = [
      writer.patch({ opacity: 0.9 }),
      writer.patch({ opacity: 1 }),
    ];
    const results = await Promise.allSettled(saves);
    expect(results.every((result) => result.status === "rejected")).toBe(true);
    expect(calls).toBe(1);
    await writer.reload(async () => ({
      value: { opacity: 0.7 },
      revision: "external",
    }));
    await writer.patch({ opacity: 0.6 });
    expect(calls).toBe(2);
  });
});
