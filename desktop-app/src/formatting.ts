import FormatWorker from "./formatting.worker?worker";
import { rpc } from "./api";
import { registerWindowFlush } from "./persistence";
import {
  FORMAT_LIMIT,
  preserveLineEndings,
  type FormatRequest,
} from "./formattingTypes";

export function formatSource(
  request: FormatRequest,
  signal: AbortSignal,
): Promise<string> {
  if (signal.aborted)
    return Promise.reject(new DOMException("Cancelled", "AbortError"));
  if (new TextEncoder().encode(request.source).length > FORMAT_LIMIT)
    return Promise.reject(new Error("Format supports up to 1 MB of source."));
  const key = `format:${crypto.randomUUID()}`;
  let cancelRequest = () => {};
  const result = new Promise<string>((resolve, reject) => {
    let worker: Worker | undefined;
    let settled = false;
    let stopReason: Error | undefined;
    const finish = (error?: unknown, text?: string) => {
      if (settled) return;
      settled = true;
      clearTimeout(timeout);
      signal.removeEventListener("abort", cancel);
      worker?.terminate();
      if (stopReason || error) reject(stopReason || error);
      else if (typeof text !== "string")
        reject(new Error("The formatter returned an invalid result."));
      else {
        const formatted = preserveLineEndings(request.source, text);
        if (new TextEncoder().encode(formatted).length > FORMAT_LIMIT)
          reject(
            new Error(
              "Formatted source exceeds 1 MB. The source was not changed.",
            ),
          );
        else resolve(formatted);
      }
    };
    const stop = (reason: Error) => {
      if (settled || stopReason) return;
      stopReason = reason;
      if (request.language === "typst") {
        // The format RPC settles after killing and reaping its helper. Keep
        // this flight alive until then so Quit cannot orphan the process.
        void rpc("cancel", { key }).catch(() => {});
      } else finish(reason);
    };
    const cancel = () => {
      stop(new DOMException("Cancelled", "AbortError"));
    };
    cancelRequest = cancel;
    const timeout = setTimeout(() => {
      stop(new Error("Formatting took too long. The source was not changed."));
    }, 10000);
    signal.addEventListener("abort", cancel, { once: true });
    if (request.language === "typst") {
      void rpc<string>("format_typst", {
        source: request.source,
        mode: request.mode,
        key,
      }).then(
        (text) => finish(undefined, text),
        (error) => finish(error),
      );
    } else {
      try {
        worker = new FormatWorker();
        worker.onmessage = (
          event: MessageEvent<{ text?: string; error?: string }>,
        ) => {
          if (event.data.error) finish(new Error(event.data.error));
          else if (typeof event.data.text === "string")
            finish(undefined, event.data.text);
          else finish(new Error("The formatter returned an invalid result."));
        };
        worker.onerror = () =>
          finish(new Error("Could not format this source."));
        worker.postMessage(request);
      } catch (error) {
        finish(error);
      }
    }
  });
  // Keep cleanup registered even if the requesting editor has unmounted.
  const unregister = registerWindowFlush(key, async () => {
    cancelRequest();
    await result.catch(() => {});
  });
  void result.then(unregister, unregister);
  return result;
}
