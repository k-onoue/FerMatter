import { beforeEach, describe, expect, it, vi } from "vitest";
const mocks = vi.hoisted(() => ({ rpc: vi.fn(), drag: vi.fn() }));
vi.mock("./api", () => ({ rpc: mocks.rpc, uid: () => "lease-id" }));
vi.mock("@crabnebula/tauri-plugin-drag", () => ({ startDrag: mocks.drag }));
import { retainArtifacts, startDrag } from "./artifactLease";

describe("artifact leases", () => {
  beforeEach(() => {
    mocks.rpc.mockReset().mockResolvedValue(null);
    mocks.drag.mockReset().mockResolvedValue(undefined);
  });
  it("keeps a native drag leased after invoke resolves, through its completion callback", async () => {
    await startDrag({ item: ["image.svg"], icon: "preview.png" });
    expect(mocks.rpc).toHaveBeenCalledTimes(1);
    expect(mocks.rpc.mock.calls[0][1].paths).toEqual([
      "image.svg",
      "preview.png",
    ]);
    mocks.drag.mock.calls[0][1]({ result: "Dropped" });
    await vi.waitFor(() => expect(mocks.rpc).toHaveBeenCalledTimes(2));
    expect(mocks.rpc.mock.calls[1][1].paths).toEqual([]);
  });
  it("orders early unmount after pending acquire, and releases exactly once", async () => {
    let acquire!: (value: null) => void;
    mocks.rpc.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          acquire = resolve;
        }),
    );
    const lease = retainArtifacts(["a.pdf"]);
    const done = lease.release();
    await lease.release();
    expect(mocks.rpc).toHaveBeenCalledTimes(1);
    acquire(null);
    await done;
    expect(mocks.rpc).toHaveBeenCalledTimes(2);
  });
  it("releases a failed native drag without hiding its error", async () => {
    mocks.drag.mockRejectedValue(new Error("drag failed"));
    await expect(startDrag({ item: ["a.svg"], icon: "a.png" })).rejects.toThrow(
      "drag failed",
    );
    await vi.waitFor(() => expect(mocks.rpc).toHaveBeenCalledTimes(2));
  });
});
