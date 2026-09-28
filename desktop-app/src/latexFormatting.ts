import { format } from "prettier/standalone";
import { prettierPluginLatex } from "@unified-latex/unified-latex-prettier";
import {
  parseMath,
  parseMinimal,
} from "@unified-latex/unified-latex-util-parse";
import type { FormatMode } from "./formattingTypes";
import { FORMAT_LIMIT } from "./formattingTypes";

// Parse bare equations in math mode, without adding/removing source delimiters.
const mathPlugin = {
  ...prettierPluginLatex,
  parsers: {
    "latex-math": {
      ...prettierPluginLatex.parsers!["latex-parser"],
      parse: (text: string) => ({
        type: "root" as const,
        content: parseMath(text),
      }),
    },
  },
};

export async function formatLatex(source: string, mode: FormatMode) {
  if (new TextEncoder().encode(source).length > FORMAT_LIMIT)
    throw new Error("Format supports up to 1 MB of source.");
  if (!source.trim()) return source;
  // The parser recovers unmatched delimiters as literal text. Reject those
  // buffers instead of letting the printer invent grouping around broken input.
  const pending = parseMinimal(source).content.map((node) => ({
    node,
    inGroup: false,
  }));
  while (pending.length) {
    const { node, inGroup } = pending.pop()!;
    if (node.type === "string" && /[{}$]/.test(node.content))
      throw new Error(
        "Close unmatched braces or math delimiters before formatting.",
      );
    if (
      node.type === "macro" &&
      (["[", "]", "(", ")"].includes(node.content) ||
        (!inGroup && ["begin", "end"].includes(node.content)))
    )
      throw new Error(
        "Close unmatched math delimiters or environments before formatting.",
      );
    if ("content" in node && Array.isArray(node.content))
      for (const child of node.content)
        pending.push({
          node: child,
          inGroup: inGroup || node.type === "group",
        });
  }
  // Explicit math environments/delimiters already carry their parsing context.
  const leading = source.replace(/^(?:\s|%[^\n]*(?:\n|$))*/, "");
  const math = mode === "math" && !/^(?:\$|\\[[(]|\\begin\s*\{)/.test(leading);
  const result = await format(source, {
    parser: math ? "latex-math" : "latex-parser",
    plugins: [math ? mathPlugin : prettierPluginLatex],
    printWidth: 80,
    tabWidth: 2,
    useTabs: false,
    endOfLine: "lf",
  });
  if (new TextEncoder().encode(result).length > FORMAT_LIMIT)
    throw new Error(
      "Formatted source exceeds 1 MB. The source was not changed.",
    );
  return result;
}
