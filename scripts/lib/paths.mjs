import { realpathSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const ROOT = fileURLToPath(new URL("../../", import.meta.url));
export const inside = (root, target) => {
  const relative = path.relative(root, target);
  return (
    !path.isAbsolute(relative) &&
    relative !== ".." &&
    !relative.startsWith(`..${path.sep}`)
  );
};
export function publicFile(root, target) {
  const actual = realpathSync(target);
  if (!inside(realpathSync(root), actual) || !statSync(actual).isFile()) {
    throw new Error(`File must stay inside its public folder: ${target}`);
  }
  return actual;
}
export function localLink(value) {
  if (/^[a-z][a-z\d+.-]*:/i.test(value) || value.startsWith("//")) return null;
  const [pathname, query = "", hash = ""] = value
    .match(/^([^?#]*)(\?[^#]*)?(#.*)?$/)
    .slice(1);
  return {
    pathname: decodeURIComponent(pathname),
    suffix: query + hash,
    fragment: decodeURIComponent(hash.slice(1)),
  };
}
export const isMain = (url) =>
  process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(url);
export function run(action) {
  try {
    action();
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
