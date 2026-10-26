import { execFileSync } from "node:child_process";
import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { tmpdir } from "node:os";

// Asset-maintenance command, not a prerequisite for normal app/site builds.
// Requires librsvg, ImageMagick, and macOS iconutil.
const root = fileURLToPath(new URL("../", import.meta.url));
const resources = path.join(root, "desktop-app/resources");
const icons = path.join(root, "desktop-app/src-tauri/icons");
const assets = path.join(root, "site/public/assets");

const mark = readFileSync(path.join(resources, "fermat.svg"), "utf8");
const blue = "#4a7699";
const white = "#f2f2e8";
const dark = "#1c2b35";
const colored = (color) =>
  mark.replace('fill="currentColor"', `fill="${color}"`);
function appIcon(source) {
  const body = source.match(/  <g[\s\S]*<\/g>/)?.[0];
  const extent = source.match(/viewBox="0 0 (\d+) (\d+)"/);
  if (!body || !extent || extent[1] !== extent[2])
    throw new Error("Expected a square Fermat SVG.");
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024">\n  <title>FerMatter</title>\n  <rect x="48" y="48" width="928" height="928" rx="208" fill="${blue}"/>\n  <g transform="translate(48 48) scale(${928 / Number(extent[1])})" fill="${white}">\n${body}\n  </g>\n</svg>\n`;
}
writeFileSync(path.join(resources, "AppIcon.svg"), appIcon(mark));
copyFileSync(
  path.join(resources, "fermat.svg"),
  path.join(assets, "fermat.svg"),
);
writeFileSync(path.join(assets, "fermat-light.svg"), colored(dark));
writeFileSync(path.join(assets, "fermat-dark.svg"), colored(white));
writeFileSync(
  path.join(assets, "favicon.svg"),
  mark.replace(
    "  <title",
    `  <style>svg{color:${dark}}@media(prefers-color-scheme:dark){svg{color:${white}}}</style>\n  <title`,
  ),
);

function render(source, destination, size) {
  execFileSync("rsvg-convert", [
    "--width",
    String(size),
    "--height",
    String(size),
    "--output",
    destination,
    source,
  ]);
}
render(path.join(resources, "AppIcon.svg"), path.join(icons, "icon.png"), 1024);
execFileSync("magick", [
  path.join(icons, "icon.png"),
  "-define",
  "icon:auto-resize=256,128,64,48,32,16",
  path.join(icons, "icon.ico"),
]);
copyFileSync(path.join(icons, "icon.png"), path.join(assets, "icon.png"));
const work = mkdtempSync(path.join(tmpdir(), "fermatter-icons-"));
try {
  const iconset = path.join(work, "FerMatter.iconset");
  mkdirSync(iconset);
  const whiteMark = path.join(work, "fermat-white.svg");
  writeFileSync(whiteMark, colored(white));
  render(
    whiteMark,
    path.join(resources, "AppIcon.icon/Assets/AppIcon.png"),
    1024,
  );
  // macOS displays this image at 20 pt (native::configure_menu_bar_icon).
  // Remove the master mark's generous margins so the face fills 42 of its
  // 44 pixels, leaving an antialiasing inset.
  const traySource = path.join(work, "tray-source.png");
  render(path.join(resources, "fermat.svg"), traySource, 256);
  execFileSync("magick", [
    traySource,
    "-trim",
    "+repage",
    "-filter",
    "Lanczos",
    "-resize",
    "42x42",
    "-gravity",
    "center",
    "-background",
    "none",
    "-extent",
    "44x44",
    "-depth",
    "8",
    "-define",
    "png:color-type=6",
    path.join(icons, "tray.png"),
  ]);
  // GitHub crops organization avatars itself; fill the entire JPEG with blue.
  execFileSync("magick", [
    path.join(icons, "icon.png"),
    "-background",
    blue,
    "-alpha",
    "remove",
    "-alpha",
    "off",
    "-colorspace",
    "sRGB",
    "-strip",
    "-quality",
    "95",
    path.join(resources, "fermatter-organization.jpg"),
  ]);

  for (const size of [16, 32, 128, 256, 512]) {
    for (const density of [1, 2]) {
      const name = `icon_${size}x${size}${density === 2 ? "@2x" : ""}.png`;
      execFileSync("magick", [
        path.join(icons, "icon.png"),
        "-filter",
        "Lanczos",
        "-resize",
        `${size * density}x${size * density}`,
        "-depth",
        "8",
        "-define",
        "png:color-type=6",
        path.join(iconset, name),
      ]);
    }
  }
  execFileSync("iconutil", [
    "--convert",
    "icns",
    "--output",
    path.join(icons, "icon.icns"),
    iconset,
  ]);
  console.log(
    "Updated app, menu-bar, website, guide, and README artwork from fermat.svg.",
  );
} finally {
  rmSync(work, { recursive: true, force: true });
}
