import templateDefaults from "./template-defaults.json";
export interface ProjectTemplate {
  id: string;
  name: string;
  format: "latex" | "typst";
  source: string;
  compiler: string;
}
export interface TemplateConfiguration {
  custom: ProjectTemplate[];
  defaultID: string;
}
export type InputMode = "automatic" | "equations" | "source";
export const builtinTemplates: ProjectTemplate[] =
  templateDefaults as ProjectTemplate[];
export const initialTemplates: TemplateConfiguration = {
  custom: [],
  defaultID: "builtin-latex",
};
export function validateTemplate(t: ProjectTemplate) {
  if (!t.name.trim()) throw new Error("Enter a template name.");
  if (t.source.split("{{content}}").length !== 2)
    throw new Error("Place {{content}} exactly once in the template.");
  if (!["latex", "typst"].includes(t.format))
    throw new Error("Choose LaTeX or Typst.");
  if (
    t.format === "latex" &&
    !["pdflatex", "xelatex", "lualatex"].includes(t.compiler)
  )
    throw new Error("Choose a LaTeX compiler.");
}
function texWithoutComments(text: string) {
  return text.replace(/(?<!\\)(?:\\\\)*%[^\n]*/g, "");
}
export function composeProject(
  text: string,
  template: ProjectTemplate,
  mode: InputMode,
): string {
  validateTemplate(template);
  if (!text.trim()) throw new Error("Paste equations or source text.");
  const normalized = text.replace(/\r\n?/g, "\n");
  const code =
    template.format === "latex"
      ? texWithoutComments(normalized)
      : normalized.replace(/\/\*[\s\S]*?\*\/|\/\/[^\n]*/g, "");
  if (
    mode !== "equations" &&
    template.format === "latex" &&
    /\\begin\s*\{document\}/.test(code)
  )
    return text;
  const looksLikeSource =
    template.format === "latex"
      ? /(?<!\\)(?:\\\\)*\$|\\[\[(]|\\(?:begin|end|section|subsection|subsubsection|chapter|part|paragraph|input|include|documentclass|usepackage|newcommand|renewcommand|providecommand|def)\b/.test(
          code,
        )
      : /(?<!\\)(?:\\\\)*\$|^\s*(?:#|={1,6}\s|```)|#(?:import|include|let|set|show|\w+\.)/m.test(
          code,
        );
  const source = mode === "source" || (mode === "automatic" && looksLikeSource);
  const content = source
    ? text
    : normalized
        .split("\n")
        .map((line) => {
          const trimmed = line.trim();
          if (
            !trimmed ||
            trimmed.startsWith(template.format === "latex" ? "%" : "//")
          )
            return line;
          return template.format === "latex"
            ? `\\[\n${line}\n\\]`
            : `$ ${line}\n $`;
        })
        .join("\n\n");
  return template.source.replace("{{content}}", content);
}
