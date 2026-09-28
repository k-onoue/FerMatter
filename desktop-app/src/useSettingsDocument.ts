import { useRef } from "react";
import { rpc } from "./api";
import { SettingsWriter } from "./settingsPersistence";

export function useSettingsDocument<T extends object>(
  file: string,
  initial: T,
  display: (value: T) => void,
  optimistic = true,
) {
  const writer = useRef<SettingsWriter<T> | null>(null);
  if (!writer.current)
    writer.current = new SettingsWriter(
      initial,
      (value, revision) => rpc<string>("settings", { file, value, revision }),
      display,
      optimistic,
    );
  return writer.current;
}
