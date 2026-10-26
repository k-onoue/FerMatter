export function isMacUserAgent(userAgent: string): boolean {
  return /Macintosh|Mac OS X/.test(userAgent);
}

// Name the system file manager from the webview's platform signal.
export function fileManagerName(userAgent: string): string {
  return isMacUserAgent(userAgent) ? "Finder" : "File Manager";
}

export const FILE_MANAGER = fileManagerName(navigator.userAgent);

// Keep the version-1 shortcut schema portable without rewriting saved settings.
// On Windows, Command denotes Ctrl and the extra Control modifier denotes Win.
export function isWindows() {
  return /Win/i.test(navigator.platform);
}

export function primaryModifier(event: { ctrlKey: boolean; metaKey: boolean }) {
  return isWindows() ? event.ctrlKey : event.metaKey;
}

export function shortcutHint(value: string) {
  return isWindows()
    ? value
        .replaceAll("⌃", "Win+")
        .replaceAll("⌥", "Alt+")
        .replaceAll("⇧", "Shift+")
        .replaceAll("⌘", "Ctrl+")
        .replaceAll("⌫", "Backspace")
        .replaceAll("↵", "Enter")
    : value;
}
