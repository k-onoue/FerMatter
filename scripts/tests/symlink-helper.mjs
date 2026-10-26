import { symlinkSync } from "node:fs";

export function symlinkOrSkip(t, target, link) {
  try {
    symlinkSync(target, link);
    return true;
  } catch (error) {
    if (process.platform === "win32" && error.code === "EPERM") {
      t.skip(
        "Windows file symlinks require Developer Mode or symlink privileges.",
      );
      return false;
    }
    throw error;
  }
}
