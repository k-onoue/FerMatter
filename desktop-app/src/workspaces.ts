import type { Card, DocumentPDF, Project } from "./types";
import type { PdfViewState } from "./PdfPane";

export type WorkspaceNavigation = {
  path: string;
  line: number;
  nonce: number;
  startUTF16?: number;
  endUTF16?: number;
};
/** Session state belongs to an open project, not to the window's settings. */
export interface WorkspaceState {
  project: Project;
  mode: "library" | "source";
  libraryStale: boolean;
  sourceMounted: boolean;
  canvasZoom: number;
  search: string;
  kind: string;
  filter: string;
  collapsed: Set<string>;
  selected: Set<string>;
  selectedAnchor?: string;
  pdfOpen: boolean;
  pdf: DocumentPDF | null;
  pdfTarget: string;
  pdfView?: PdfViewState;
  navigate?: WorkspaceNavigation;
  drafts: Record<string, string>;
  editing: Card | null;
  editPreview: Card | null;
  inspector: boolean;
  scrollTop: number;
}

export function newWorkspace(project: Project): WorkspaceState {
  return {
    project,
    mode: "library",
    libraryStale: false,
    sourceMounted: false,
    canvasZoom: 1,
    search: "",
    kind: "all",
    filter: "all",
    collapsed: new Set(),
    selected: new Set(),
    pdfOpen: false,
    pdf: null,
    pdfTarget: "",
    drafts: {},
    editing: (project.libraryDraft as Card | undefined) || null,
    editPreview: null,
    inspector: false,
    scrollTop: 0,
  };
}
