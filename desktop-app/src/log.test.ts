import { expect, it } from "vitest";
import { formatLogMessage, logError } from "./log";

it("prefixes the source and stringifies non-Error values", () => {
  expect(formatLogMessage("report", "boom")).toBe("[report] boom");
  expect(formatLogMessage("report", 42)).toBe("[report] 42");
  expect(formatLogMessage("report", new Error("bad"))).toContain("bad");
});

it("truncates long messages to 2000 characters", () => {
  expect(formatLogMessage("report", "x".repeat(5000))).toHaveLength(2000);
});

it("never throws outside Tauri", () => {
  expect(() => logError("test", new Error("outside tauri"))).not.toThrow();
});
