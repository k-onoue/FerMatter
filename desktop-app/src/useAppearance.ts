import { useEffect } from "react";
import { palette } from "./appearance";
import { configureTheme } from "./editor";
import type { Theme } from "./types";

export function useAppearance(theme: Theme, dark: boolean, opacity: number) {
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
}
