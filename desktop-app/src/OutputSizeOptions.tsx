import { outputSizeOptions } from "./outputSizes";

export function OutputSizeOptions({
  value,
  sourceSizes,
  projectSize,
}: {
  value: number;
  sourceSizes: (number | undefined)[];
  projectSize?: number;
}) {
  return outputSizeOptions(value, sourceSizes, projectSize).map((option) => (
    <option key={option.value} value={option.value} title={option.title}>
      {option.label}
    </option>
  ));
}
