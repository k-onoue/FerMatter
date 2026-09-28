import type { Theme, Palette } from "./types";
export const defaults: Theme = {
  schemaVersion: 1,
  kind: "mathlidelux.theme",
  id: "org.mathlidelux.default",
  name: "Default",
  light: {
    accent: "#007AFF",
    windowBackground: "#EEF0F4",
    surfaceBackground: "#FFFFFF",
    text: "#20252D",
    command: "#8835B5",
    delimiter: "#078C94",
    number: "#BA5B00",
    comment: "#76818B",
  },
  dark: {
    accent: "#2997FF",
    windowBackground: "#1D232C",
    surfaceBackground: "#242B35",
    text: "#E7EDF5",
    command: "#C486F5",
    delimiter: "#65CCD0",
    number: "#F5B271",
    comment: "#8895A5",
  },
};
export function palette(theme: Theme | undefined, dark: boolean): Palette {
  const mode = dark ? "dark" : "light";
  const colors = { ...defaults[mode] };
  for (const [k, v] of Object.entries(theme?.[mode] || {}))
    if (/^#[0-9a-f]{6}$/i.test(v)) colors[k as keyof Palette] = v;
  return colors;
}
