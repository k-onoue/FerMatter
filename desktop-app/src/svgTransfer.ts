/** Read drop representations without inserting markup or fetching remote URLs. */
const SVG_LIMIT = 16 * 1024 * 1024;
const TOTAL_LIMIT = 64 * 1024 * 1024;
const countLimit = 16;
export interface SVGTransfer {
  files: {
    name: string;
    type: string;
    size: number;
    text(): Promise<string>;
  }[];
  svg: string;
  html: string;
  text: string;
}
const bytes = (text: string) => new TextEncoder().encode(text).length;
function bounded(values: string[]) {
  if (
    values.length > countLimit ||
    values.some((text) => bytes(text) > SVG_LIMIT)
  )
    throw new Error("Drop up to 16 SVG files, each smaller than 16 MB.");
  if (values.reduce((total, text) => total + bytes(text), 0) > TOTAL_LIMIT)
    throw new Error("SVG selection exceeds 64 MB.");
  return values;
}
function svgDataURL(url: string): string | null {
  const match = /^data:image\/svg\+xml((?:;[^,]*)?),(.*)$/is.exec(url);
  if (!match) return null;
  if (match[2].length > TOTAL_LIMIT)
    throw new Error("SVG selection exceeds 64 MB.");
  if (/(?:^|;)base64(?:;|$)/i.test(match[1])) {
    const binary = atob(match[2].replace(/\s/g, ""));
    return new TextDecoder("utf-8", { fatal: true }).decode(
      Uint8Array.from(binary, (character) => character.charCodeAt(0)),
    );
  }
  return decodeURIComponent(match[2]);
}
export function htmlSVGs(html: string): string[] {
  if (html.length > TOTAL_LIMIT || bytes(html) > TOTAL_LIMIT)
    throw new Error("SVG selection exceeds 64 MB.");
  // Only extract self-contained data. Never mount the HTML, resolve file URLs,
  // or load linked artwork from the network.
  const inline: string[] = [];
  const opening = /<svg\b/gi,
    closing = /<\/svg\s*>/gi;
  let match: RegExpExecArray | null;
  // Separate forward searches avoid quadratic backtracking on repeated,
  // unclosed <svg tokens in malformed external HTML.
  while ((match = opening.exec(html))) {
    closing.lastIndex = opening.lastIndex;
    const end = closing.exec(html);
    if (!end) break;
    inline.push(html.slice(match.index, closing.lastIndex));
    bounded(inline);
    opening.lastIndex = closing.lastIndex;
  }
  if (inline.length) return bounded(inline);
  const values: string[] = [];
  for (const match of html.matchAll(
    /\b(?:src|data)\s*=\s*(["'])(data:image\/svg\+xml[^"']*)\1/gi,
  )) {
    const source = svgDataURL(
      match[2]
        .replace(/&amp;/g, "&")
        .replace(/&#39;/g, "'")
        .replace(/&quot;/g, '"'),
    );
    if (source !== null) values.push(source);
    bounded(values);
  }
  return values;
}
export function acceptsSVG(transfer: SVGTransfer): boolean {
  return (
    !!transfer.svg ||
    (transfer.files.length > 0 &&
      transfer.files.every(
        (file) => /\.svg$/i.test(file.name) || file.type === "image/svg+xml",
      )) ||
    /<svg\b|data:image\/svg\+xml/i.test(transfer.html) ||
    /^\s*(?:<\?xml[^>]*>\s*)?<svg\b|^data:image\/svg\+xml/i.test(transfer.text)
  );
}
export async function readSVGTransfer(
  transfer: SVGTransfer,
): Promise<string[]> {
  if (transfer.svg) return bounded([transfer.svg]);
  if (
    transfer.files.length &&
    transfer.files.every(
      (file) => /\.svg$/i.test(file.name) || file.type === "image/svg+xml",
    )
  ) {
    if (
      transfer.files.length > countLimit ||
      transfer.files.some((file) => file.size > SVG_LIMIT)
    )
      throw new Error("Drop up to 16 SVG files, each smaller than 16 MB.");
    if (
      transfer.files.reduce((total, file) => total + file.size, 0) > TOTAL_LIMIT
    )
      throw new Error("SVG selection exceeds 64 MB.");
    return bounded(
      await Promise.all(transfer.files.map((file) => file.text())),
    );
  }
  const html = htmlSVGs(transfer.html);
  if (html.length) return html;
  if (transfer.text.length > TOTAL_LIMIT)
    throw new Error("SVG selection exceeds 64 MB.");
  const data = svgDataURL(transfer.text);
  if (data !== null) return bounded([data]);
  return bounded([transfer.text]);
}
