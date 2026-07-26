import React, { useRef } from "react";
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
  const lastPointerType = useRef<string | null>(null);

  return (
    <div className="flex h-full min-h-[72px] w-full items-center justify-center px-2 sm:min-h-[80px] sm:px-4">
      <div
        className={`flex h-full w-full cursor-pointer touch-manipulation flex-col items-center justify-center rounded-md border bg-card p-2 text-center text-sm font-medium text-card-foreground shadow-sm transition-colors hover:border-primary/50 sm:p-3 ${
          isSelected
            ? "border-primary ring-2 ring-primary/20 bg-primary/10"
            : "border-border"
        }`}
        onPointerDown={(e) => {
          e.stopPropagation();
          lastPointerType.current = e.pointerType;
        }}
        onClick={() => {
          if (lastPointerType.current === "touch" && isSelected) {
            onDoubleClick(personId);
          } else {
            onSelect(personId);
          }
          lastPointerType.current = null;
        }}
        onDoubleClick={() => onDoubleClick(personId)}
      >
        {name}
      </div>
    </div>
  );
};
