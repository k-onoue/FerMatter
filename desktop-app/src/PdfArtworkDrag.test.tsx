// @vitest-environment jsdom
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { PdfArtworkDrag } from "./PdfArtworkDrag";
import type { DocumentPDF, Project } from "./types";
const { rpc } = vi.hoisted(() => ({ rpc: vi.fn() }));
vi.mock("./api", () => ({ rpc }));
vi.mock("./artifactLease", () => ({
  startDrag: vi.fn(),
  useCardArtifacts: vi.fn(),
}));
afterEach(cleanup);
it("re-prepares PDF artwork when the inherited default changes", async () => {
  rpc.mockImplementation(async (op: string) =>
    op === "pdf_prepare_drag"
      ? { card: { id: "eq" }, path: "preview.svg" }
      : null,
  );
  const props = {
    anchor: { targetID: "eq", page: 1, x: 10, y: 10, width: 40, height: 20 },
    document: { pdfPath: "paper.pdf", targets: [] } as unknown as DocumentPDF,
    project: { id: "p", cards: [] } as unknown as Project,
    scale: 1,
    onNavigate: vi.fn(),
  };
  const view = render(<PdfArtworkDrag {...props} defaultFontSize={30} />);
  await screen.findByRole("button", { name: "Drag Equation" });
  expect(rpc).toHaveBeenCalledWith(
    "pdf_prepare_drag",
    expect.objectContaining({ fontSize: 30 }),
  );
  view.rerender(<PdfArtworkDrag {...props} defaultFontSize={31.25} />);
  await waitFor(() =>
    expect(rpc).toHaveBeenCalledWith(
      "pdf_prepare_drag",
      expect.objectContaining({ fontSize: 31.25 }),
    ),
  );
  expect(rpc).toHaveBeenCalledWith("cancel", { key: "p:pdf-drag" });
});
