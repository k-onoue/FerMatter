import config from "./site.json";
import pages from "../../../user-docs/pages.json";
export const href = (path = "") => `${config.base}/${path.replace(/^\//, "")}`;
export const repository = `https://github.com/${config.repository}`;
export const releases = `https://github.com/${config.releaseRepository}/releases`;
export const download = `${releases}/download/${config.releaseTag}/${config.assetName}`;
const guideSlugs = new Set(
  pages.map((page) => (page === "README.md" ? "" : page.replace(/\.md$/, ""))),
);
export const guide = (slug = "", section = "") => {
  if (!guideSlugs.has(slug)) throw new Error(`Guide is not published: ${slug}`);
  return (
    href(`guide/${slug ? `${slug}/` : ""}`) + (section ? `#${section}` : "")
  );
};
