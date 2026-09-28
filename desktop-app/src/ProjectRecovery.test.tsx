// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { ProjectRecovery, type ProjectProblem } from "./ProjectRecovery";
const { rpc } = vi.hoisted(() => ({ rpc: vi.fn() }));
vi.mock("./api", () => ({ rpc, asset: vi.fn() }));
const problem: ProjectProblem = {
  id: "synthetic",
  path: "/synthetic/metadata/project.json",
  message: "Invalid or incomplete project JSON.",
  canRestore: true,
  revision: "reviewed-revision",
};
beforeEach(() => {
  rpc.mockReset();
});
afterEach(cleanup);
it("restores only after explicit action and uses the reviewed metadata revision", async () => {
  const onProblems = vi.fn(),
    onProjects = vi.fn();
  render(
    <ProjectRecovery
      problems={[problem]}
      onProblems={onProblems}
      onProjects={onProjects}
    />,
  );
  expect(rpc).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "Review recovery" }));
  expect(screen.getByText(problem.path)).toBeTruthy();
  expect(rpc).not.toHaveBeenCalled();
  const restored = [{ id: "synthetic", root: "/synthetic/source" }];
  rpc.mockResolvedValue({ projects: restored, problems: [] });
  fireEvent.click(
    screen.getByRole("button", { name: "Restore last valid backup" }),
  );
  await waitFor(() => expect(onProjects).toHaveBeenCalledWith(restored));
  expect(rpc).toHaveBeenCalledWith("recover_project", {
    project: problem.id,
    revision: problem.revision,
  });
  expect(onProblems).toHaveBeenCalledWith([]);
});
it("keeps failed recovery visible without exposing backend error contents", async () => {
  const onProblems = vi.fn(),
    onProjects = vi.fn();
  rpc.mockRejectedValue(Error("PRIVATE_SOURCE_SENTINEL"));
  render(
    <ProjectRecovery
      problems={[problem]}
      onProblems={onProblems}
      onProjects={onProjects}
    />,
  );
  fireEvent.click(screen.getByRole("button", { name: "Review recovery" }));
  fireEvent.click(
    screen.getByRole("button", { name: "Restore last valid backup" }),
  );
  const alert = await screen.findByRole("alert");
  expect(alert.textContent).toContain("original metadata has been kept");
  expect(document.body.textContent).not.toContain("PRIVATE_SOURCE_SENTINEL");
  expect(screen.getByText(problem.path)).toBeTruthy();
  expect(onProjects).not.toHaveBeenCalled();
  expect(onProblems).not.toHaveBeenCalled();
});
