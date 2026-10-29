import { UpdatesButton } from "./Updates";
import type { ReactNode } from "react";

export function WindowControls({ children }: { children: ReactNode }) {
  return (
    <div className="window-controls" role="group" aria-label="Window controls">
      <UpdatesButton />
      {children}
    </div>
  );
}
