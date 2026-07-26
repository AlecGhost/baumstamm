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
      <div className="w-full h-full flex flex-col items-center justify-center gap-4">
        <p className="text-muted-foreground">
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
        <div className="p-16">
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
        className="absolute top-4 left-4 max-w-sm rounded-md border border-border bg-card p-3 shadow-sm"
        onPointerDown={(event) => event.stopPropagation()}
        onPointerUp={(event) => event.stopPropagation()}
      >
        <div className="flex items-center justify-between gap-4">
          <div>
            <p className="text-sm font-medium">Tree view</p>
            <p className="text-xs text-muted-foreground" aria-live="polite">
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
            >
              Full tree
            </Button>
          )}
        </div>
        <div
          className="mt-2 flex gap-1"
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
              >
                {label}
              </Button>
            );
          })}
        </div>
      </div>

      {/* Controls overlay */}
      <div
        className="absolute bottom-4 right-4 flex gap-2 bg-card border border-border rounded-md shadow-sm p-1"
        onPointerDown={(e) => e.stopPropagation()}
        onPointerUp={(e) => e.stopPropagation()}
      >
        <button
          onClick={() => setZoom((z) => Math.max(z * 0.8, 0.1))}
          className="p-2 hover:bg-muted rounded"
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
          className="p-2 hover:bg-muted rounded text-xs font-medium"
          title="Reset View"
        >
          100%
        </button>
        <div className="w-px bg-border my-1" />
        <button
          onClick={() => setZoom((z) => Math.min(z * 1.25, 5))}
          className="p-2 hover:bg-muted rounded"
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
