import React, { useState, useEffect } from "react";
import {
  getPersonName,
  type TreeData,
  type TreeViewScope,
  type TreeViewSelection,
} from "@/lib/types";
import { TreeGrid } from "./TreeGrid";
import { PersonDetailsModal } from "./PersonDetailsModal";
import { usePanZoom } from "@/hooks/use-pan-zoom";
import { Button } from "@/components/ui/button";
import { Plus } from "lucide-react";

interface TreeCanvasProps {
  data: TreeData | null;
  viewSelection: TreeViewSelection | null;
  onCreate: () => void;
  onUpdate: () => void;
  onSetPartialView: (root: string, scope: TreeViewScope) => void;
  onSetFullView: () => void;
}

export const TreeCanvas: React.FC<TreeCanvasProps> = ({
  data,
  viewSelection,
  onCreate,
  onUpdate,
  onSetPartialView,
  onSetFullView,
}) => {
  const [selectedPersonId, setSelectedPersonId] = useState<string | null>(null);
  const [isModalOpen, setIsModalOpen] = useState(false);

  const { containerRef, setZoom, setPan, pointerHandlers, transformStyle } =
    usePanZoom({ enabled: !isModalOpen });

  // Handle Enter key for selected person
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Enter" && selectedPersonId && !isModalOpen) {
        setIsModalOpen(true);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [selectedPersonId, isModalOpen]);

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
              setSelectedPersonId((prev) => (prev === id ? null : id))
            }
            onDoubleClickPerson={(id: string) => {
              setSelectedPersonId(id);
              setIsModalOpen(true);
            }}
          />
        </div>
      </div>

      <div
        className="absolute left-3 right-3 top-3 max-w-sm rounded-md border border-border bg-card p-3 shadow-sm sm:left-4 sm:right-auto sm:top-4"
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
          {viewSelection && (
            <Button
              type="button"
              size="sm"
              variant="outline"
              onClick={onSetFullView}
              className="h-10 shrink-0 sm:h-8"
            >
              Full tree
            </Button>
          )}
        </div>
        <div
          className="mt-2 grid grid-cols-3 gap-1"
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
      </div>

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
        onSelectPerson={setSelectedPersonId}
      />
    </div>
  );
};
