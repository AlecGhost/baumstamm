import React from "react";
import type { Person } from "@/lib/types";
import { getPersonName } from "@/lib/types";

interface PersonCellProps {
  personId: string;
  person?: Person;
  isSelected: boolean;
  onSelect: (id: string) => void;
  onDoubleClick: (id: string) => void;
}

export const PersonCell: React.FC<PersonCellProps> = ({
  personId,
  person,
  isSelected,
  onSelect,
  onDoubleClick,
}) => {
  const name = person ? getPersonName(person) : "Unknown Person";

  return (
    <div className="w-full h-full min-h-[80px] p-2 flex items-center justify-center">
      <div
        className={`bg-card text-card-foreground border rounded-md shadow-sm w-full h-full flex flex-col items-center justify-center p-3 text-sm font-medium hover:border-primary/50 transition-colors cursor-pointer ${
          isSelected
            ? "border-primary ring-2 ring-primary/20 bg-primary/10"
            : "border-border"
        }`}
        onPointerDown={(e) => e.stopPropagation()}
        onClick={() => onSelect(personId)}
        onDoubleClick={() => onDoubleClick(personId)}
      >
        {name}
      </div>
    </div>
  );
};
