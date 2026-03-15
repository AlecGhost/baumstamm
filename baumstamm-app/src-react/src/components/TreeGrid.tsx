import React from "react";
import type { TreeData } from "@/lib/types";
import { PersonCell } from "./PersonCell";
import { ConnectionCell } from "./ConnectionCell";

interface TreeGridProps {
  data: TreeData;
  selectedPersonId: string | null;
  onSelectPerson: (id: string) => void;
  onDoubleClickPerson: (id: string) => void;
}

export const TreeGrid: React.FC<TreeGridProps> = ({
  data,
  selectedPersonId,
  onSelectPerson,
  onDoubleClickPerson,
}) => {
  const { grid, persons } = data;

  if (!grid || grid.length === 0) {
    return null;
  }

  const rows = grid.length;
  const cols = grid[0].length;

  // Find person object by ID
  const getPerson = (id: string) => persons.find((p) => p.id === id);

  return (
    <div
      className="grid gap-0 place-items-center"
      style={{
        gridTemplateColumns: `repeat(${cols}, minmax(180px, 1fr))`,
        gridTemplateRows: `repeat(${rows}, auto)`,
      }}
    >
      {grid.map((row, rowIndex) =>
        row.map((cell, colIndex) => {
          const key = `${rowIndex}-${colIndex}`;

          if ("Person" in cell) {
            return (
              <div key={key} className="w-full h-full p-2">
                <PersonCell
                  personId={cell.Person}
                  person={getPerson(cell.Person)}
                  isSelected={cell.Person === selectedPersonId}
                  onSelect={onSelectPerson}
                  onDoubleClick={onDoubleClickPerson}
                />
              </div>
            );
          } else if ("Connections" in cell) {
            return (
              <div key={key} className="w-full h-full min-h-[40px]">
                <ConnectionCell connections={cell.Connections} />
              </div>
            );
          }

          return <div key={key} />;
        }),
      )}
    </div>
  );
};
