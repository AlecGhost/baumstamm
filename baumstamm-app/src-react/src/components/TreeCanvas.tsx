import React, { useCallback, useEffect, useState } from "react";
import {
  getPersonName,
  type GridLayoutAlgorithm,
  type TreeData,
  type TreeViewScope,
  type TreeViewSelection,
  type ViewOptions,
} from "@/lib/types";
import { updateViewOption } from "@/lib/view-options";
import {
  getTreeNavigationTarget,
  type TreeNavigationDirection,
} from "@/lib/tree-navigation";
import { TreeGrid } from "./TreeGrid";
import { PersonDetailsModal } from "./PersonDetailsModal";
import { usePanZoom } from "@/hooks/use-pan-zoom";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { LoaderCircle, Network, Plus, Save } from "lucide-react";

interface TreeCanvasProps {
  data: TreeData | null;
  viewSelection: TreeViewSelection | null;
  viewOptions: ViewOptions;
  gridLayoutAlgorithm: GridLayoutAlgorithm;
  isTreeViewUpdating: boolean;
  isGridLayoutUpdating: boolean;
  isSubTreeSaving: boolean;
  onCreate: () => void;
  onUpdate: () => void;
  onSetPartialView: (root: string, scope: TreeViewScope) => void;
  onViewOptionsChange: (options: ViewOptions) => void;
  onGridLayoutChange: (layoutAlgorithm: GridLayoutAlgorithm) => void;
  onSaveSubTree: () => void;
}

export const TreeCanvas: React.FC<TreeCanvasProps> = ({
  data,
  viewSelection,
  viewOptions,
  gridLayoutAlgorithm,
  isTreeViewUpdating,
  isGridLayoutUpdating,
  isSubTreeSaving,
  onCreate,
  onUpdate,
  onSetPartialView,
  onViewOptionsChange,
  onGridLayoutChange,
  onSaveSubTree,
}) => {
  const [selectedPersonId, setSelectedPersonId] = useState<string | null>(null);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [isViewPanelExpanded, setIsViewPanelExpanded] = useState(false);

  const { containerRef, setZoom, setPan, pointerHandlers, transformStyle } =
    usePanZoom({ enabled: !isModalOpen });

  const selectPerson = useCallback((id: string | null) => {
    setSelectedPersonId(id);
  }, []);

  const centerSelectedPerson = useCallback(() => {
    if (!selectedPersonId) return false;

    const container = containerRef.current;
    const personElement = container?.querySelector<HTMLElement>(
      `[data-tree-person-id="${CSS.escape(selectedPersonId)}"]`,
    );
    if (!container || !personElement) return false;

    const containerBounds = container.getBoundingClientRect();
    const personBounds = personElement.getBoundingClientRect();
    const delta = {
      x:
        containerBounds.left +
        containerBounds.width / 2 -
        (personBounds.left + personBounds.width / 2),
      y:
        containerBounds.top +
        containerBounds.height / 2 -
        (personBounds.top + personBounds.height / 2),
    };
    setPan((current) => ({
      x: current.x + delta.x,
      y: current.y + delta.y,
    }));
    return true;
  }, [containerRef, selectedPersonId, setPan]);

  const navigateSelection = useCallback(
    (direction: TreeNavigationDirection) => {
      if (!data || !selectedPersonId) return false;

      const nextPersonId = getTreeNavigationTarget(
        data,
        selectedPersonId,
        direction,
      );
      if (!nextPersonId) return false;

      setSelectedPersonId(nextPersonId);
      return true;
    },
    [data, selectedPersonId],
  );

  // Handle keyboard navigation for the main tree.
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (isModalOpen) return;

      const target = e.target;
      if (
        target instanceof HTMLElement &&
        (["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName) ||
          target.isContentEditable ||
          target.closest('[contenteditable="true"]'))
      ) {
        return;
      }

      if (e.key === "Enter" && selectedPersonId) {
        e.preventDefault();
        setIsModalOpen(true);
        return;
      }

      if (e.altKey || e.ctrlKey || e.metaKey || e.shiftKey) return;

      if (e.key.toLowerCase() === "c" && centerSelectedPerson()) {
        e.preventDefault();
      } else {
        const navigationHandled =
          (e.key === "ArrowUp" && navigateSelection("up")) ||
          (e.key === "ArrowDown" && navigateSelection("down")) ||
          (e.key === "ArrowLeft" && navigateSelection("left")) ||
          (e.key === "ArrowRight" && navigateSelection("right"));
        if (navigationHandled) {
          e.preventDefault();
        }
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [centerSelectedPerson, navigateSelection, selectedPersonId, isModalOpen]);

  if (!data) {
    return (
      <div className="flex h-full w-full flex-col items-center justify-center gap-4 px-6 text-center">
        <p className="max-w-sm text-sm text-muted-foreground sm:text-base">
          No tree loaded. Create a new family tree or load an existing one.
        </p>
        <Button onClick={onCreate}>
          <Plus className="h-4 w-4" />
          Create New Tree
        </Button>
      </div>
    );
  }

  const selectedPerson = selectedPersonId
    ? data.persons.find((person) => person.id === selectedPersonId)
    : undefined;
  const viewRoot = viewSelection
    ? data.persons.find((person) => person.id === viewSelection.root)
    : undefined;
  const scopeLabels: Record<TreeViewScope, string> = {
    ancestors: "Ancestors",
    descendants: "Descendants",
    both: "Both",
  };

  return (
    <div
      ref={containerRef}
      className="w-full h-full overflow-hidden bg-background relative cursor-grab active:cursor-grabbing select-none"
      aria-label="Family tree. Select a person, use arrow keys to navigate relatives, C to center, and Enter to open details."
      aria-keyshortcuts="C ArrowUp ArrowDown ArrowLeft ArrowRight Enter"
      {...pointerHandlers}
    >
      <div
        className="absolute origin-center transition-transform duration-75 ease-out"
        style={transformStyle}
      >
        <div className="p-6 sm:p-16">
          <TreeGrid
            data={data}
            selectedPersonId={selectedPersonId}
            onSelectPerson={(id: string) =>
              selectPerson(selectedPersonId === id ? null : id)
            }
            onDoubleClickPerson={(id: string) => {
              selectPerson(id);
              setIsModalOpen(true);
            }}
          />
        </div>
      </div>

      {isViewPanelExpanded ? (
        <div
          className="absolute left-3 top-3 max-h-[calc(100%-1.5rem)] w-[calc(100%-1.5rem)] max-w-sm overflow-y-auto rounded-md border border-border bg-card p-3 shadow-sm sm:left-4 sm:top-4 sm:max-h-[calc(100%-2rem)]"
          onPointerDown={(event) => event.stopPropagation()}
          onPointerUp={(event) => event.stopPropagation()}
        >
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <p className="text-sm font-medium">Tree view</p>
              <p
                className="line-clamp-2 text-xs text-muted-foreground"
                aria-live="polite"
              >
                {viewSelection
                  ? `${scopeLabels[viewSelection.scope]} of ${getPersonName(viewRoot)}`
                  : selectedPerson
                    ? `Choose relatives of ${getPersonName(selectedPerson)}`
                    : "Select a person to filter the tree"}
              </p>
            </div>
            <Button
              type="button"
              size="icon"
              variant="ghost"
              className="h-9 w-9 shrink-0"
              aria-label="Collapse tree view options"
              aria-expanded="true"
              onClick={() => setIsViewPanelExpanded(false)}
            >
              <Network className="h-4 w-4" aria-hidden="true" />
            </Button>
          </div>

          <fieldset
            className="mt-2 disabled:cursor-wait disabled:opacity-60"
            disabled={isTreeViewUpdating}
            aria-busy={isTreeViewUpdating}
          >
            <legend className="sr-only">Tree relationship options</legend>
            <div
              className="grid grid-cols-3 gap-1"
              role="group"
              aria-label={
                selectedPerson
                  ? `Filter tree around ${getPersonName(selectedPerson)}`
                  : "Filter tree around selected person"
              }
            >
              {(
                [
                  ["ancestors", "Ancestors"],
                  ["descendants", "Descendants"],
                  ["both", "Both"],
                ] as const
              ).map(([scope, label]) => {
                const isActive =
                  viewSelection?.root === selectedPersonId &&
                  viewSelection.scope === scope;
                return (
                  <Button
                    key={scope}
                    type="button"
                    size="sm"
                    variant={isActive ? "default" : "outline"}
                    disabled={!selectedPersonId}
                    aria-pressed={isActive}
                    onClick={() => {
                      if (selectedPersonId) {
                        onSetPartialView(selectedPersonId, scope);
                      }
                    }}
                    className="h-10 min-w-0 px-1 text-xs sm:h-8 sm:px-3 sm:text-sm"
                  >
                    {label}
                  </Button>
                );
              })}
            </div>

            <div className="mt-3 grid grid-cols-2 gap-x-3 gap-y-2 border-t border-border pt-3">
              {(
                [
                  ["show_partners", "Partners"],
                  ["show_siblings", "Siblings"],
                  ["show_partner_siblings", "Partner siblings"],
                  ["show_ancestor_siblings", "Ancestor siblings"],
                ] as const
              ).map(([key, label]) => (
                <label
                  key={key}
                  className="flex min-h-8 items-center gap-2 text-xs"
                >
                  <input
                    type="checkbox"
                    checked={viewOptions[key]}
                    onChange={(event) =>
                      onViewOptionsChange(
                        updateViewOption(
                          viewOptions,
                          key,
                          event.currentTarget.checked,
                        ),
                      )
                    }
                  />
                  <span>{label}</span>
                </label>
              ))}
            </div>

            <Button
              type="button"
              size="sm"
              variant="outline"
              className="mt-3 w-full"
              disabled={!viewSelection || isSubTreeSaving}
              onClick={onSaveSubTree}
            >
              {isSubTreeSaving ? (
                <LoaderCircle
                  className="h-4 w-4 animate-spin"
                  aria-hidden="true"
                />
              ) : (
                <Save className="h-4 w-4" aria-hidden="true" />
              )}
              {isSubTreeSaving ? "Saving sub-tree…" : "Save sub-tree"}
            </Button>
          </fieldset>

          <Separator className="my-3" />

          <fieldset
            disabled={isGridLayoutUpdating}
            aria-busy={isGridLayoutUpdating}
            className="disabled:cursor-wait disabled:opacity-60"
          >
            <legend className="sr-only">Grid layout options</legend>
            <label className="grid grid-cols-[minmax(0,1fr)_9rem] items-center gap-2 text-xs">
              <span>Grid layout</span>
              <span className="relative">
                <select
                  className="h-8 w-full rounded-md border border-input bg-background px-2 text-xs"
                  value={gridLayoutAlgorithm}
                  aria-label="Grid layout algorithm"
                  onChange={(event) =>
                    onGridLayoutChange(
                      event.currentTarget.value as GridLayoutAlgorithm,
                    )
                  }
                >
                  <option value="Centered">Centered</option>
                  <option value="ConnectionOptimized">
                    Connection optimized
                  </option>
                  <option value="ForceDirected">Relationship forces</option>
                  <option value="KinshipExpansion">Kinship expansion</option>
                </select>
                {isGridLayoutUpdating && (
                  <LoaderCircle
                    className="pointer-events-none absolute right-7 top-2 h-4 w-4 animate-spin"
                    aria-hidden="true"
                  />
                )}
              </span>
            </label>
          </fieldset>
        </div>
      ) : (
        <Button
          type="button"
          size="icon"
          variant="outline"
          className="absolute left-3 top-3 h-11 w-11 bg-card shadow-sm sm:left-4 sm:top-4"
          aria-label="Expand tree view options"
          aria-expanded="false"
          onClick={() => setIsViewPanelExpanded(true)}
          onPointerDown={(event) => event.stopPropagation()}
          onPointerUp={(event) => event.stopPropagation()}
        >
          <Network className="h-5 w-5" aria-hidden="true" />
        </Button>
      )}

      {/* Controls overlay */}
      <div
        className="absolute bottom-3 right-3 flex gap-1 rounded-md border border-border bg-card p-1 shadow-sm sm:bottom-4 sm:right-4 sm:gap-2"
        onPointerDown={(e) => e.stopPropagation()}
        onPointerUp={(e) => e.stopPropagation()}
      >
        <button
          onClick={() => setZoom((z) => Math.max(z * 0.8, 0.1))}
          className="flex h-11 w-11 items-center justify-center rounded hover:bg-muted sm:h-auto sm:w-auto sm:p-2"
          aria-label="Zoom out"
          title="Zoom Out"
        >
          <svg
            width="15"
            height="15"
            viewBox="0 0 15 15"
            fill="none"
            xmlns="http://www.w3.org/2000/svg"
          >
            <path
              d="M4 7.5C4 7.22386 4.22386 7 4.5 7H10.5C10.7761 7 11 7.22386 11 7.5C11 7.77614 10.7761 8 10.5 8H4.5C4.22386 8 4 7.77614 4 7.5Z"
              fill="currentColor"
              fillRule="evenodd"
              clipRule="evenodd"
            ></path>
          </svg>
        </button>
        <div className="w-px bg-border my-1" />
        <button
          onClick={() => {
            setZoom(1);
            setPan({ x: 0, y: 0 });
          }}
          className="flex h-11 min-w-12 items-center justify-center rounded px-2 text-xs font-medium hover:bg-muted sm:h-auto sm:min-w-0 sm:p-2"
          title="Reset View"
        >
          100%
        </button>
        <div className="w-px bg-border my-1" />
        <button
          onClick={() => setZoom((z) => Math.min(z * 1.25, 5))}
          className="flex h-11 w-11 items-center justify-center rounded hover:bg-muted sm:h-auto sm:w-auto sm:p-2"
          aria-label="Zoom in"
          title="Zoom In"
        >
          <svg
            width="15"
            height="15"
            viewBox="0 0 15 15"
            fill="none"
            xmlns="http://www.w3.org/2000/svg"
          >
            <path
              d="M8 4.5C8 4.22386 7.77614 4 7.5 4C7.22386 4 7 4.22386 7 4.5V7H4.5C4.22386 7 4 7.22386 4 7.5C4 7.77614 4.22386 8 4.5 8H7V10.5C7 10.7761 7.22386 11 7.5 11C7.77614 11 8 10.7761 8 10.5V8H10.5C10.7761 8 11 7.77614 11 7.5C11 7.22386 10.7761 7 10.5 7H8V4.5Z"
              fill="currentColor"
              fillRule="evenodd"
              clipRule="evenodd"
            ></path>
          </svg>
        </button>
      </div>

      <PersonDetailsModal
        person={selectedPerson ?? null}
        treeData={data}
        isOpen={isModalOpen}
        onClose={() => setIsModalOpen(false)}
        onUpdate={onUpdate}
        onSelectPerson={selectPerson}
      />
    </div>
  );
};
