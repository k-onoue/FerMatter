import { listen } from "@tauri-apps/api/event";
import { rpc } from "./api";
import { flushWindow } from "./persistence";

export async function installWindowPersistence(
  report: (error: unknown) => void,
) {
  const stopError = await listen<string>("persistence-error", ({ payload }) =>
    report(payload),
  );
  const stopFlush = await listen<{ token: number }>(
    "flush-window",
    async ({ payload }) => {
      let error = "";
      document.documentElement.inert = true;
      try {
        await flushWindow();
      } catch (failure) {
        error = String(failure);
        report(failure);
      }
      await rpc("window_flush_complete", { token: payload.token, error }).catch(
        report,
      );
      document.documentElement.inert = false;
    },
  );
  return () => {
    stopError();
    stopFlush();
  };
}
