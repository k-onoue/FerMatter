import { useEffect } from "react";
import { palette } from "./appearance";
import { configureTheme } from "./editor";
import { rpc } from "./api";
import { logError } from "./log";
import { isMacOS, isWindows } from "./platform";
import type { Theme } from "./types";

export function useAppearance(theme: Theme, dark: boolean, opacity: number) {
  const background = palette(theme, dark).windowBackground;
  useEffect(() => {
    const colors = palette(theme, dark);
    configureTheme(colors, dark);
    document.documentElement.dataset.appearance = dark ? "dark" : "light";
    for (const [key, value] of Object.entries(colors))
      document.documentElement.style.setProperty("--" + key, value);
    document.documentElement.style.setProperty("--opacity", String(opacity));
    // Keep WebKit's canvas transparent: root color-scheme would paint an opaque
    // background. Only surfaces use opacity; text remains fully opaque.
  }, [theme, dark, opacity]);

  useEffect(() => {
    if (isMacOS()) return;
    const applyNative = () => {
      void rpc("apply_window_appearance", { dark, background }).catch((error) =>
        logError("window.appearance", error),
      );
    };
    applyNative();
    // Windows may restore its system menu colors when another native window
    // closes or becomes inactive. Reapply this window's current app palette.
    if (isWindows()) {
      window.addEventListener("focus", applyNative);
      return () => window.removeEventListener("focus", applyNative);
    }
  }, [dark, background]);
}
