import { describe, it, expect } from "vitest";
import {
  builtinTemplates,
  composeProject,
  validateTemplate,
} from "./templates";
describe("pasted project content", () => {
  it("wraps individual bare equations and preserves blank/comment lines", () => {
    const result = composeProject(
      "x^2\r\n% note\r\ny^2",
      builtinTemplates[0],
      "automatic",
    );
    expect(result).toContain("\\[\nx^2\n\\]\n\n% note\n\n\\[\ny^2\n\\]");
  });
  it("preserves a complete TeX document byte for byte", () => {
    const source =
      "\\documentclass{article}\r\n\\begin{document}\r\n$x$\r\n\\end{document}\r\n";
    expect(composeProject(source, builtinTemplates[2], "automatic")).toBe(
      source,
    );
  });
  it("does not misclassify commented-out document delimiters", () => {
    const result = composeProject(
      "% \\begin{document}\nx",
      builtinTemplates[0],
      "automatic",
    );
    expect(result).toContain("\\[\nx\n\\]");
  });
  it("keeps Typst setup and markup in source mode", () => {
    const input = "#let norm(x) = $ ||x|| $\n= Geometry\n$ norm(x) $";
    expect(composeProject(input, builtinTemplates[1], "automatic")).toContain(
      input,
    );
  });
  it("requires a single insertion point", () => {
    expect(() =>
      validateTemplate({
        ...builtinTemplates[0],
        source: "{{content}}{{content}}",
      }),
    ).toThrow();
  });
});
