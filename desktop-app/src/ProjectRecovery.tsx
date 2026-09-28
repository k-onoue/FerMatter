import { useState } from "react";
import { rpc } from "./api";
import { Modal } from "./ui";
import type { Project } from "./types";

export type ProjectProblem = {
  id: string;
  path: string;
  message: string;
  canRestore: boolean;
  revision: string | null;
};
export function ProjectRecovery({
  problems,
  onProblems,
  onProjects,
}: {
  problems: ProjectProblem[];
  onProblems: (problems: ProjectProblem[]) => void;
  onProjects: (projects: Project[]) => void;
}) {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  async function run(problem?: ProjectProblem) {
    setBusy(true);
    setError("");
    try {
      if (problem) {
        const result = await rpc<{
          projects: Project[];
          problems: ProjectProblem[];
        }>("recover_project", {
          project: problem.id,
          revision: problem.revision,
        });
        onProjects(result.projects);
        onProblems(result.problems);
      } else onProblems(await rpc<ProjectProblem[]>("project_problems"));
    } catch {
      // Do not include parser input, source text or an exception stack in this view.
      setError(
        "Recovery could not complete. The original metadata has been kept. Refresh and try again.",
      );
    } finally {
      setBusy(false);
    }
  }
  return (
    <>
      {problems.length > 0 && (
        <div className="project-recovery-notice" role="status">
          <span>
            {problems.length} project metadata{" "}
            {problems.length === 1 ? "file needs" : "files need"} attention.
          </span>
          <button onClick={() => setOpen(true)}>Review recovery</button>
        </div>
      )}
      {open && (
        <Modal
          title="Project recovery"
          onClose={() => {
            if (!busy) setOpen(false);
          }}
        >
          <p>
            Other projects remain available. Restoring a backup keeps a separate
            copy of the unreadable metadata; source files and drafts stay in
            place.
          </p>
          {problems.map((problem) => (
            <section key={problem.path} className="project-recovery-item">
              <code>{problem.path}</code>
              <p>{problem.message}</p>
              {problem.canRestore ? (
                <button disabled={busy} onClick={() => void run(problem)}>
                  Restore last valid backup
                </button>
              ) : (
                <p>
                  No valid automatic backup is available. Keep this file for
                  manual recovery.
                </p>
              )}
            </section>
          ))}
          {problems.length === 0 && <p>No metadata problems found.</p>}
          {error && <p role="alert">{error}</p>}
          <footer>
            <button disabled={busy} onClick={() => void run()}>
              Refresh
            </button>
            <button disabled={busy} onClick={() => setOpen(false)}>
              Done
            </button>
          </footer>
        </Modal>
      )}
    </>
  );
}
