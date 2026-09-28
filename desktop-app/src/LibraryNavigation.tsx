import type { Dispatch, SetStateAction } from "react";
import { ChevronDown, ChevronRight, Grid2X2, Plus, Tag } from "lucide-react";
import type { Project } from "./types";
export function LibraryNavigation({
  project,
  filter,
  setFilter,
  collapsed,
  setCollapsed,
  onOutline,
  onNewLabel,
}: {
  project: Project;
  filter: string;
  setFilter: (value: string) => void;
  collapsed: Set<string>;
  setCollapsed: Dispatch<SetStateAction<Set<string>>>;
  onOutline: (id: string) => void;
  onNewLabel: () => void;
}) {
  return (
    <div className="library-navigation">
      <button
        className={`nav-row ${filter === "all" ? "selected" : ""}`}
        onClick={() => setFilter("all")}
      >
        <Grid2X2 size={16} />
        <span>All equations</span>
        <small>{project.cards.length}</small>
      </button>
      <div className="nav-heading">Outline</div>
      {project.outline
        ?.filter((h) => !h.path.slice(0, -1).some((id) => collapsed.has(id)))
        .map((h) => (
          <div
            key={h.id}
            className={`outline-row ${filter === h.id ? "selected" : ""}`}
            style={{ paddingLeft: 8 + (h.path.length - 1) * 13 }}
          >
            <button
              title="Collapse section"
              className="disclosure"
              onClick={() =>
                setCollapsed((old) => {
                  const next = new Set(old);
                  next.has(h.id) ? next.delete(h.id) : next.add(h.id);
                  return next;
                })
              }
            >
              {project.outline.some(
                (n) => n.path.includes(h.id) && n.id !== h.id,
              ) ? (
                collapsed.has(h.id) ? (
                  <ChevronRight size={12} />
                ) : (
                  <ChevronDown size={12} />
                )
              ) : null}
            </button>
            <button
              onClick={() => {
                setFilter(h.id);
                onOutline(h.id);
              }}
            >
              {h.title}
            </button>
            <small>
              {
                project.cards.filter((c) => c.outlinePath?.includes(h.id))
                  .length
              }
            </small>
          </div>
        ))}
      <div className="nav-heading">
        Labels
        <button title="New label" onClick={onNewLabel}>
          <Plus size={15} />
        </button>
      </div>
      <button
        className={`nav-row ${filter === "unlabeled" ? "selected" : ""}`}
        onClick={() => setFilter("unlabeled")}
      >
        <Tag size={15} />
        <span>Unlabeled</span>
      </button>
      {project.sections?.map((s) => (
        <button
          className={`nav-row ${filter === s.id ? "selected" : ""}`}
          key={s.id}
          onClick={() => setFilter(s.id)}
        >
          <Tag size={15} />
          <span>{s.name}</span>
        </button>
      ))}
    </div>
  );
}
