import { expect, it } from "vitest";
import { outputSizeOptions } from "./outputSizes";

it("shows the source point size without rewriting legacy automatic values", () => {
  expect(
    outputSizeOptions(0, [11]).find((option) => option.value === 0)?.label,
  ).toBe("11 pt");
  const options = outputSizeOptions(0, [12]);
  expect(options.filter((option) => option.label === "12 pt")).toEqual([
    { value: 0, label: "12 pt", title: "Source font size" },
  ]);
  expect(
    outputSizeOptions(12, [12]).find((option) => option.value === 12)?.label,
  ).toBe("12 pt");
});
it("retains imported sizes and distinguishes mixed or unavailable source sizes", () => {
  expect(
    outputSizeOptions(18, [11]).find((option) => option.value === 18)?.label,
  ).toBe("18 pt");
  expect(outputSizeOptions(0, [10, 11, 11, 12.5])[0].label).toBe("10–12.5 pt");
  expect(outputSizeOptions(0, [undefined, NaN, 0])[0].label).toBe("—");
});
it("keeps project inheritance and explicit source sizes separate", () => {
  const options = outputSizeOptions(-1, [11], 24);
  expect(options.find((option) => option.value === -1)?.label).toBe(
    "Project default (24 pt)",
  );
  expect(options.find((option) => option.value === 0)?.label).toBe("11 pt");
  expect(outputSizeOptions(-1, [11], 0)[0].label).toBe(
    "Project default (11 pt)",
  );
});
