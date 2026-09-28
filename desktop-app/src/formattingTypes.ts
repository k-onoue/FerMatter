export type FormatMode = "document" | "math";
export type FormatLanguage = "latex" | "typst";
export interface FormatRequest {
  source: string;
  language: FormatLanguage;
  mode: FormatMode;
}
export const FORMAT_LIMIT = 1024 * 1024;

/** Source buffers own their line-ending convention, even after formatting. */
export function preserveLineEndings(source: string, formatted: string) {
  const lf = formatted.replace(/\r\n?/g, "\n");
  const finalNewline = /(?:\r\n|\r|\n)$/.test(source);
  const result = lf.replace(/\n+$/, "") + (finalNewline ? "\n" : "");
  return source.includes("\r\n") ? result.replace(/\n/g, "\r\n") : result;
}
