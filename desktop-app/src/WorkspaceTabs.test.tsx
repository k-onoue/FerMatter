// @vitest-environment jsdom
import { useState } from "react";
import { afterEach, expect, it, vi } from "vitest";
import {
  cleanup,
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { WorkspaceTabs } from "./WorkspaceTabs";
import { newWorkspace } from "./workspaces";
import type { Project } from "./types";
import { lockWindowInteraction } from "./persistence";
vi.mock("./api", () => ({ projectName: (root: string) => root.slice(1) }));
vi.mock("./artifactLease", () => ({ useCardArtifacts: () => {} }));
afterEach(cleanup);
it("moves focus with successive project-tab arrow keys", async () => {
  const workspaces = ["First", "Second"].map((id) =>
    newWorkspace({ id, root: `/${id}` } as Project),
  );
  function Tabs() {
    const [active, setActive] = useState<string | null>(null);
    return (
      <WorkspaceTabs
        workspaces={workspaces}
        active={active}
        onSelect={setActive}
        onClose={() => {}}
      />
    );
  }
  render(<Tabs />);
  const home = screen.getByRole("tab", { name: "Projects" });
  home.focus();
  fireEvent.keyDown(home, { key: "ArrowRight" });
  await waitFor(() =>
    expect(document.activeElement).toBe(
      screen.getByRole("tab", { name: "First" }),
    ),
  );
  fireEvent.keyDown(document.activeElement!, { key: "ArrowRight" });
  await waitFor(() =>
    expect(document.activeElement).toBe(
      screen.getByRole("tab", { name: "Second" }),
    ),
  );
  fireEvent.keyDown(document.activeElement!, { key: "Home" });
  await waitFor(() => expect(document.activeElement).toBe(home));
});

it("defers arrow focus until the tab transition unlocks and cancels an unmounted request", async () => {
  const workspaces = [newWorkspace({ id: "First", root: "/First" } as Project)];
  let release!: () => void;
  function Tabs() {
    const [active, setActive] = useState<string | null>(null);
    return (
      <WorkspaceTabs
        workspaces={workspaces}
        active={active}
        onSelect={(id) => {
          release = lockWindowInteraction();
          setActive(id);
        }}
        onClose={() => {}}
      />
    );
  }
  const view = render(<Tabs />);
  const home = screen.getByRole("tab", { name: "Projects" });
  const project = screen.getByRole("tab", { name: "First" });
  const focus = vi.spyOn(project, "focus");
  home.focus();
  fireEvent.keyDown(home, { key: "ArrowRight" });
  expect(project.getAttribute("aria-selected")).toBe("true");
  expect(focus).not.toHaveBeenCalled();
  act(() => release());
  expect(focus).toHaveBeenCalledOnce();
  expect(document.activeElement).toBe(project);
  const homeFocus = vi.spyOn(home, "focus");
  fireEvent.keyDown(project, { key: "ArrowLeft" });
  expect(homeFocus).not.toHaveBeenCalled();
  view.unmount();
  act(() => release());
  expect(homeFocus).not.toHaveBeenCalled();
});
