import { useEffect, useRef } from "react";
import { FolderOpen, Grid2X2, X } from "lucide-react";
import { projectName } from "./api";
import { retainArtifacts, useCardArtifacts } from "./artifactLease";
import type { WorkspaceState } from "./workspaces";
import { runWhenWindowInteractive } from "./persistence";

export function WorkspaceTabs({
  workspaces,
  active,
  onSelect,
  onClose,
}: {
  workspaces: WorkspaceState[];
  active: string | null;
  onSelect: (id: string | null) => void;
  onClose: (id: string) => void;
}) {
  const ids = [null, ...workspaces.map((workspace) => workspace.project.id)];
  const buttons = useRef(new Map<string, HTMLButtonElement>());
  const focusAfterSelection = useRef<string | null>(null);
  useEffect(() => {
    const id = active || "home";
    const button = buttons.current.get(id);
    button?.scrollIntoView?.({ block: "nearest", inline: "nearest" });
    if (focusAfterSelection.current === id)
      return runWhenWindowInteractive(() => {
        if (focusAfterSelection.current !== id) return;
        button?.focus({ preventScroll: true });
        focusAfterSelection.current = null;
      });
  }, [active]);
  return (
    <nav className="workspace-tabs" role="tablist" aria-label="Projects">
      {ids.map((id, index) => {
        const project = index > 0 ? workspaces[index - 1].project : null;
        const name = project ? projectName(project.root) : "Projects";
        return (
          <div
            key={id || "home"}
            className={`workspace-tab ${active === id ? "active" : ""}`}
          >
            <button
              ref={(element) => {
                if (element) buttons.current.set(id || "home", element);
                else buttons.current.delete(id || "home");
              }}
              role="tab"
              aria-selected={active === id}
              tabIndex={active === id ? 0 : -1}
              title={project?.root || "Projects"}
              onClick={() => onSelect(id)}
              onAuxClick={(event) => {
                if (event.button === 1 && id) {
                  event.preventDefault();
                  onClose(id);
                }
              }}
              onKeyDown={(event) => {
                let next: number | undefined;
                if (event.key === "ArrowRight") next = (index + 1) % ids.length;
                if (event.key === "ArrowLeft")
                  next = (index + ids.length - 1) % ids.length;
                if (event.key === "Home") next = 0;
                if (event.key === "End") next = ids.length - 1;
                if (next !== undefined) {
                  event.preventDefault();
                  focusAfterSelection.current = ids[next] || "home";
                  onSelect(ids[next]);
                }
              }}
            >
              {project ? <FolderOpen size={14} /> : <Grid2X2 size={14} />}
              <span>{name}</span>
            </button>
            {id && (
              <button
                className="workspace-tab-close"
                aria-label={`Close ${name}`}
                title={`Close ${name}`}
                onClick={() => onClose(id)}
              >
                <X size={13} />
              </button>
            )}
          </div>
        );
      })}
    </nav>
  );
}

/** Inactive tabs still own the PDF/editor snapshots restored on selection. */
export function WorkspaceArtifacts({
  workspace,
}: {
  workspace: WorkspaceState;
}) {
  useCardArtifacts(workspace.editPreview);
  const pdf = workspace.pdf;
  useEffect(() => {
    if (!pdf) return;
    const lease = retainArtifacts([pdf.pdfPath, pdf.sourceRoot]);
    void lease.acquired.catch(console.error);
    return () => {
      void lease.release().catch(console.error);
    };
  }, [pdf?.pdfPath, pdf?.sourceRoot]);
  return null;
}
