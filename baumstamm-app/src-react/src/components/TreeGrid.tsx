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

  // A row is considered empty if every cell is either a bare empty cell or
  // a Connections cell whose passing, ending, and crossing arrays are all empty.
  const isEmptyRow = (row: (typeof grid)[number]) =>
    row.every((cell) => {
      if ("Person" in cell) return false;
      if ("Connections" in cell) {
        const c = cell.Connections;
        return (
          c.passing.length === 0 &&
          c.ending.length === 0 &&
          c.crossing.length === 0
        );
      }
      return true; // empty cell
    });

  // Trim empty rows from the start and end
  let startRow = 0;
  while (startRow < grid.length && isEmptyRow(grid[startRow])) startRow++;
  let endRow = grid.length - 1;
  while (endRow > startRow && isEmptyRow(grid[endRow])) endRow--;

  const trimmedGrid = grid.slice(startRow, endRow + 1);

  const rows = trimmedGrid.length;
  const cols = trimmedGrid.length > 0 ? trimmedGrid[0].length : 0;

  if (rows === 0) return null;

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
      {trimmedGrid.map((row, rowIndex) =>
        row.map((cell, colIndex) => {
          const key = `${startRow + rowIndex}-${colIndex}`;

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
