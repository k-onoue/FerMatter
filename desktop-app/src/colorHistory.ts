const storageKey = "mathlidelux.color-history.v1";
const limit = 16;

function validColors(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return [
    ...new Set(
      value
        .filter(
          (color): color is string =>
            typeof color === "string" && /^#[0-9a-f]{6}$/i.test(color),
        )
        .map((color) => color.toLowerCase()),
    ),
  ].slice(0, limit);
}

export function loadColorHistory(): string[] {
  try {
    return validColors(JSON.parse(localStorage.getItem(storageKey) || "[]"));
  } catch {
    return [];
  }
}

export function rememberColor(color: string, fallback: string[]): string[] {
  // Read again so a choice in another Instant window is not overwritten.
  const history = validColors([color, ...loadColorHistory(), ...fallback]);
  try {
    localStorage.setItem(storageKey, JSON.stringify(history));
  } catch {
    // Picking and rendering colors still works if browser storage is unavailable.
  }
  return history;
}
