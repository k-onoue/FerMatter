import type { ReactNode } from "react";

export function WindowTitlebar({ children }: { children?: ReactNode }) {
  return (
    <div className="titlebar">
      <div className="titlebar-drag-region" data-tauri-drag-region />
      {children && (
        <div
          className="window-controls"
          role="group"
          aria-label="Window controls"
        >
          {children}
        </div>
      )}
    </div>
  );
}
