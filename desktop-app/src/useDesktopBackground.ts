import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { rpc } from "./api";

export function useDesktopBackground() {
  const [image, setImage] = useState<string>();
  useEffect(() => {
    let gone = false;
    let sequence = 0;
    let key = "";
    const unlisten: (() => void)[] = [];
    async function refresh() {
      const ticket = ++sequence;
      try {
        const result = await rpc<{ key: string; image: string | null } | null>(
          "desktop_background",
          { key },
        );
        if (gone || ticket !== sequence) return;
        if (!result) {
          key = "";
          setImage(undefined);
        } else {
          key = result.key;
          if (result.image) setImage(result.image);
        }
      } catch {
        // Never leave an old display's image behind after a failed refresh.
        if (!gone && ticket === sequence) {
          key = "";
          setImage(undefined);
        }
      }
    }
    const win = getCurrentWindow();
    for (const subscribe of [
      win.onResized.bind(win),
      win.onMoved.bind(win),
      win.onFocusChanged.bind(win),
    ]) {
      void subscribe(() => void refresh()).then((stop) =>
        gone ? stop() : unlisten.push(stop),
      );
    }
    void refresh();
    const timer = setInterval(() => void refresh(), 10_000);
    return () => {
      gone = true;
      sequence++;
      clearInterval(timer);
      unlisten.forEach((stop) => stop());
    };
  }, []);
  return image;
}
