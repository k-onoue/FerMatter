import { listen } from "@tauri-apps/api/event";
import { rpc } from "./api";
import { flushWindow, lockWindowInteraction } from "./persistence";

export async function installWindowPersistence(
  report: (error: unknown) => void,
) {
  const leases = new Map<number, () => void>();
  let finishedThrough = 0;
  const stopFinished = await listen<{ token: number }>(
    "flush-window-finished",
    ({ payload }) => {
      finishedThrough = Math.max(finishedThrough, payload.token);
      for (const [token, release] of leases) {
        if (token <= payload.token) {
          release();
          leases.delete(token);
        }
      }
    },
  );
  const stopError = await listen<string>("persistence-error", ({ payload }) =>
    report(payload),
  );
  const stopFlush = await listen<{ token: number }>(
    "flush-window",
    async ({ payload }) => {
      if (payload.token <= finishedThrough || leases.has(payload.token)) return;
      const release = lockWindowInteraction();
      leases.set(payload.token, release);
      try {
        let error = "";
        try {
          await flushWindow();
        } catch (failure) {
          error = String(failure);
          report(failure);
        }
        await rpc("window_flush_complete", {
          token: payload.token,
          error,
        }).catch(report);
      } catch (failure) {
        report(failure);
      }
      // A successful acknowledgement can still be waiting for another window.
      // Native completion/cancellation releases the lease; Quit keeps it to exit.
    },
  );
  return () => {
    stopError();
    stopFlush();
    stopFinished();
    for (const release of leases.values()) release();
    leases.clear();
  };
}
