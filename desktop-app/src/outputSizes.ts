const presets = [12, 16, 20, 24, 32, 40, 48, 60, 72];
const valid = (size: number | undefined): size is number =>
  size !== undefined && Number.isFinite(size) && size > 0;
const points = (size: number) => String(Number(size.toFixed(2)));

export function outputSizeOptions(
  value: number,
  sourceSizes: (number | undefined)[],
  projectSize?: number,
) {
  const originals = [...new Set(sourceSizes.filter(valid))].sort(
    (a, b) => a - b,
  );
  const sourceLabel = !originals.length
    ? "—"
    : originals.length === 1
      ? `${points(originals[0])} pt`
      : `${points(originals[0])}–${points(originals.at(-1)!)} pt`;
  const options: { value: number; label: string; title?: string }[] = [];
  if (projectSize !== undefined)
    options.push({
      value: -1,
      label: `Project default (${projectSize > 0 ? points(projectSize) + " pt" : sourceLabel})`,
    });
  // Keep legacy automatic sizes intact. Only the label changes; choosing a
  // different number remains an explicit output-size change.
  if (originals.length !== 1 || value !== originals[0])
    options.push({ value: 0, label: sourceLabel, title: "Source font size" });
  const sizes = [
    ...new Set([...presets, ...originals, ...(value > 0 ? [value] : [])]),
  ];
  for (const size of sizes.sort((a, b) => a - b)) {
    if (originals.length === 1 && size === originals[0] && value !== size)
      continue;
    options.push({ value: size, label: `${points(size)} pt` });
  }
  return options;
}
