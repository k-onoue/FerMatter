export function isHexColor(value: unknown): value is string {
  return typeof value === "string" && /^#[0-9a-f]{6}$/i.test(value);
}

export function parseHexInput(value: string): string | null {
  const color = value.startsWith("#") ? value : "#" + value;
  return isHexColor(color) ? color.toLowerCase() : null;
}

/** Saved cards use six digits without '#'; do not rewrite their legacy field. */
export function cardOutputColor(value: unknown): string | null {
  const color = typeof value === "string" ? "#" + value : null;
  return isHexColor(color) ? color : null;
}

export function isBrightColor(color: string): boolean {
  return (
    0.2126 * parseInt(color.slice(1, 3), 16) +
      0.7152 * parseInt(color.slice(3, 5), 16) +
      0.0722 * parseInt(color.slice(5, 7), 16) >
    220
  );
}
