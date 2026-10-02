import { error } from "@tauri-apps/plugin-log";

const MAX_LOG_MESSAGE = 2000;

export function formatLogMessage(source: string, e: unknown): string {
  const detail = e instanceof Error ? (e.stack ?? e.message) : String(e);
  return `[${source}] ${detail}`.slice(0, MAX_LOG_MESSAGE);
}

export function logError(source: string, e: unknown): void {
  try {
    void error(formatLogMessage(source, e)).catch(() => {});
  } catch {
    // Logging must never surface errors, including outside Tauri.
  }
}
