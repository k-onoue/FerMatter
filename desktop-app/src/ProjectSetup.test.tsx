// @vitest-environment jsdom
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { ProjectSetup } from "./ProjectSetup";

const { open, rpc, saveTools } = vi.hoisted(() => ({
  open: vi.fn(),
  rpc: vi.fn(),
  saveTools: vi.fn(),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open }));
vi.mock("./api", () => ({ rpc }));
vi.mock("./CompilerTools", () => ({
  CompilerToolFields: () => null,
  useCompilerTools: () => ({ draft: {}, busy: false, save: saveTools }),
}));
beforeEach(() => {
  open.mockReset().mockResolvedValue(null);
  rpc.mockReset().mockResolvedValue({ settings: {}, projects: [] });
  saveTools.mockReset().mockResolvedValue(true);
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

it("uses the Windows default without requiring a folder picker", async () => {
  vi.spyOn(navigator, "platform", "get").mockReturnValue("Win32");
  const directory = "C:\\Users\\Example\\Documents\\FerMatter";
  const onComplete = vi.fn();
  render(<ProjectSetup initialDirectory={directory} onComplete={onComplete} />);
  expect(screen.getByText(directory)).toBeTruthy();
  expect(
    (screen.getByRole("button", { name: "Next" }) as HTMLButtonElement)
      .disabled,
  ).toBe(false);
  fireEvent.click(screen.getByRole("button", { name: "Next" }));
  fireEvent.click(screen.getByRole("button", { name: "Finish Setup" }));
  await waitFor(() => expect(onComplete).toHaveBeenCalled());
  expect(open).not.toHaveBeenCalled();
  expect(rpc).toHaveBeenCalledWith("complete_project_setup", { directory });
});

it("keeps a redirected Windows default on cancel and preserves a custom choice", async () => {
  vi.spyOn(navigator, "platform", "get").mockReturnValue("Win32");
  const onComplete = vi.fn();
  const directory = "C:\\Users\\Example\\OneDrive\\Documents\\FerMatter";
  const view = render(
    <ProjectSetup initialDirectory={directory} onComplete={onComplete} />,
  );
  fireEvent.click(screen.getByRole("button", { name: "Choose Folder…" }));
  await waitFor(() =>
    expect(open).toHaveBeenCalledWith(
      expect.objectContaining({ defaultPath: directory }),
    ),
  );
  expect(screen.getByText(directory)).toBeTruthy();
  open.mockResolvedValueOnce("D:\\Work");
  fireEvent.click(screen.getByRole("button", { name: "Choose Folder…" }));
  await screen.findByText("D:\\Work");
  view.rerender(
    <ProjectSetup
      initialDirectory="E:\\Documents\\FerMatter"
      onComplete={onComplete}
    />,
  );
  expect(screen.getByText("D:\\Work")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Next" }));
  fireEvent.click(screen.getByRole("button", { name: "Finish Setup" }));
  await waitFor(() =>
    expect(rpc).toHaveBeenCalledWith("complete_project_setup", {
      directory: "D:\\Work",
    }),
  );
});

it.each(["MacIntel", "Linux x86_64"])(
  "retains the explicit folder choice on %s",
  async (platform) => {
    vi.spyOn(navigator, "platform", "get").mockReturnValue(platform);
    const directory = "/example/Documents/Mathlidelux";
    render(<ProjectSetup initialDirectory={directory} onComplete={vi.fn()} />);
    expect(screen.queryByText(directory)).toBeNull();
    expect(
      (screen.getByRole("button", { name: "Next" }) as HTMLButtonElement)
        .disabled,
    ).toBe(true);
    open.mockResolvedValueOnce(directory);
    fireEvent.click(screen.getByRole("button", { name: "Choose Folder…" }));
    await screen.findByText(directory);
    expect(
      (screen.getByRole("button", { name: "Next" }) as HTMLButtonElement)
        .disabled,
    ).toBe(false);
  },
);
