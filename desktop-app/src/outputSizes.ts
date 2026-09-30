export const DEFAULT_OUTPUT_SIZE = 30;

// Zero is a saved legacy choice to preserve the source size, not inheritance.
export function outputSizeText(
  size: number,
  sourceSizes: (number | undefined)[] = [],
) {
  if (size > 0) return String(size);
  const originals = [
    ...new Set(
      sourceSizes.filter(
        (n): n is number => n !== undefined && Number.isFinite(n) && n > 0,
      ),
    ),
  ];
  return originals.length === 1 ? String(Number(originals[0].toFixed(2))) : "";
}
