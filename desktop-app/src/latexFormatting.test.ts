import { expect, it } from "vitest";
import { formatLatex } from "./latexFormatting";
import { FORMAT_LIMIT, preserveLineEndings } from "./formattingTypes";

it("formats bare equations without adding math delimiters", async () => {
  const formatted = await formatLatex(
    String.raw`\frac { a+b } {c} + x^2`,
    "math",
  );
  expect(formatted).toContain(String.raw`\frac{ a+b }{c}`);
  expect(formatted).toContain("x^{2}");
  expect(formatted).not.toMatch(/\$|\\\[|\\\]/);
  expect(await formatLatex(formatted, "math")).toBe(formatted);
});
it("retains explicit math, comments, custom definitions, and verbatim", async () => {
  const source = String.raw`\documentclass{article}
\usepackage{amsmath}
\newcommand{\pair}[2]{\left(#1,#2\right)}
\begin{document}
% project definitions
\[ \pair{x}{y} + \frac{1}{2} \]
\begin{verbatim}
x   =   { y
\end{verbatim}
\end{document}`;
  const formatted = await formatLatex(source, "document");
  expect(formatted).toContain(
    String.raw`\newcommand{\pair}[2]{\left(#1,#2\right)}`,
  );
  expect(formatted).toContain("% project definitions");
  expect(formatted).toContain("x   =   { y\n");
  expect(formatted).toContain(String.raw`\pair{x}{y}`);
  expect(await formatLatex(formatted, "document")).toBe(formatted);
  expect(await formatLatex("$x^2$", "math")).toBe("$x^{2}$");
});
it("preserves alignment rows, TikZ, and escaped delimiters", async () => {
  const source = String.raw`\begin{aligned}x&=y+1\\z&=3\end{aligned}`;
  const formatted = await formatLatex(source, "math");
  expect(formatted).toContain("\n");
  expect(formatted).toContain(String.raw`\\`);
  expect(await formatLatex(formatted, "math")).toBe(formatted);
  const tikz = String.raw`\begin{tikzpicture}\draw[blue] (0,0)--(1,1);\node at (0,0) {\$\{x\}};\end{tikzpicture}`;
  const result = await formatLatex(tikz, "document");
  expect(result).toContain(String.raw`\$\{x\}`);
  expect(await formatLatex(result, "document")).toBe(result);
});
it("does not consume a trailing comment or silently repair incomplete delimiters", async () => {
  expect(await formatLatex("x=1 % keep\n+y", "math")).toMatch(/% keep\n/);
  for (const text of [
    String.raw`\frac{a}{`,
    "x}",
    "$x",
    String.raw`\(\frac {a}{b}`,
    String.raw`\begin{aligned}x&=y`,
  ])
    await expect(formatLatex(text, "math")).rejects.toThrow("unmatched");
  expect(await formatLatex("\n  ", "math")).toBe("\n  ");
  await expect(
    formatLatex("x".repeat(FORMAT_LIMIT + 1), "math"),
  ).rejects.toThrow("1 MB");
});
it("bounds formatting expansion before returning a replacement", async () => {
  const source =
    "\\begin{itemize}\n".repeat(200) +
    "\\item a long synthetic item with multiple words and formatting.\n".repeat(
      2000,
    ) +
    "\\end{itemize}\n".repeat(200);
  expect(source.length).toBeLessThan(FORMAT_LIMIT);
  await expect(formatLatex(source, "document")).rejects.toThrow("exceeds 1 MB");
});
it("retains CRLF and the presence of a final newline", () => {
  expect(preserveLineEndings("x\r\ny\r\n", "x\ny\n\n")).toBe("x\r\ny\r\n");
  expect(preserveLineEndings("x\r\ny", "x\ny\n")).toBe("x\r\ny");
  expect(preserveLineEndings("x", "x\n")).toBe("x");
  expect(preserveLineEndings("x\n", "x")).toBe("x\n");
});
