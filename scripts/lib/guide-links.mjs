import path from "node:path";
import { readFileSync } from "node:fs";
import { ROOT, publicFile, localLink } from "./paths.mjs";

// Keep Markdown links useful in GitHub, while validating/rebasing their public URLs.
export function guideLink(
  value,
  source,
  { root = ROOT, base = "/FerMatter" } = {},
) {
  const url = localLink(value);
  if (!url?.pathname) return value;
  const sourceDir = path.join(root, "user-docs");
  const pages = JSON.parse(
    readFileSync(path.join(sourceDir, "pages.json"), "utf8"),
  );
  const target = path.resolve(path.dirname(source), url.pathname);
  const name = pages.find((name) => path.resolve(sourceDir, name) === target);
  if (name) {
    publicFile(sourceDir, target);
    return `${base}/guide/${name === "README.md" ? "" : name.replace(/\.md$/, "") + "/"}${url.suffix}`;
  }
  const downloads = path.join(root, "docs/downloads");
  if (target.startsWith(downloads + path.sep)) {
    publicFile(downloads, target);
    return `${base}/downloads/${encodeURIComponent(path.basename(target))}${url.suffix}`;
  }
  const assets = path.join(root, "site/public");
  publicFile(assets, target);
  return `${base}/${path.relative(assets, target).split(path.sep).map(encodeURIComponent).join("/")}${url.suffix}`;
}
export function remarkGuideLinks({ base = "/FerMatter" } = {}) {
  return (tree, file) => {
    if (!file.path?.includes(`${path.sep}user-docs${path.sep}`)) return;
    function walk(node) {
      if (["link", "image", "definition"].includes(node.type))
        node.url = guideLink(node.url, file.path, { base });
      node.children?.forEach(walk);
    }
    walk(tree);
  };
}
