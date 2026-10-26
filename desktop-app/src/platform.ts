// The webview user agent is the only platform signal the frontend has.
export function fileManagerName(userAgent: string): string {
  return /Macintosh|Mac OS X/.test(userAgent) ? "Finder" : "File Manager";
}

export const FILE_MANAGER = fileManagerName(navigator.userAgent);
