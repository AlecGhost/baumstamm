import React, { useEffect, useMemo, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/tauri";
import {
  ArrowDown,
  ArrowUp,
  ArrowUpDown,
  Columns3,
  Plus,
  UserRound,
} from "lucide-react";
import {
  getPersonName,
  type Person,
  type TreeData,
  type TreeViewScope,
  type TreeViewSelection,
} from "@/lib/types";
import { Button } from "@/components/ui/button";
import { PersonDetailsModal } from "@/components/PersonDetailsModal";

interface PersonTableProps {
  data: TreeData | null;
  fullTreePersons: Person[];
  viewSelection: TreeViewSelection | null;
  onCreate: () => void;
  onUpdate: () => void;
  onSetPartialView: (root: string, scope: TreeViewScope) => void;
  onSetFullView: () => void;
}

type SortDirection = "ascending" | "descending";

type Column = {
  id: string;
  label: string;
  getValue: (person: Person) => string;
};

const NAME_COLUMN_ID = "builtin:name";
const infoColumnId = (key: string) => `info:${key}`;
const defaultColumnIds = [
  NAME_COLUMN_ID,
  infoColumnId("@dateOfBirth"),
  infoColumnId("@dateOfDeath"),
];
const reservedInfoKeys = new Set(["@firstName", "@lastName", "@image"]);

const infoKeyLabel = (key: string) => {
  if (key === "@dateOfBirth") return "Date of birth";
  if (key === "@dateOfDeath") return "Date of death";

  const words = key
    .replace(/^@/, "")
    .replace(/[_-]+/g, " ")
    .replace(/([a-z\d])([A-Z])/g, "$1 $2")
    .trim();
  return words
    ? `${words.charAt(0).toUpperCase()}${words.slice(1)}`
    : "Unnamed property";
};

const displayImageSource = (person: Person) => {
  const source = person.info?.get("@image")?.trim();
  if (!source) return undefined;

  const isLocalAbsolutePath =
    source.startsWith("/") || /^[A-Za-z]:[\\/]/.test(source);
  if (!isLocalAbsolutePath) return source;
  if (typeof window === "undefined" || !("__TAURI__" in window)) {
    return undefined;
  }

  try {
    return convertFileSrc(source);
  } catch {
    return undefined;
  }
};

const PersonPreview: React.FC<{ person: Person }> = ({ person }) => {
  const source = displayImageSource(person);
  const [failedSource, setFailedSource] = useState<string | undefined>();
  const name = getPersonName(person);
  const canShowImage = source !== undefined && source !== failedSource;

  return (
    <span className="flex h-9 w-9 shrink-0 items-center justify-center overflow-hidden rounded-full bg-primary/10 text-sm font-semibold text-primary/60">
      {canShowImage ? (
        <img
          src={source}
          alt=""
          className="h-full w-full object-cover"
          onError={() => setFailedSource(source)}
        />
      ) : name === "Unknown" ? (
        <UserRound className="h-4 w-4" aria-hidden="true" />
      ) : (
        <span aria-hidden="true">{name.charAt(0).toLocaleUpperCase()}</span>
      )}
    </span>
  );
};

export const PersonTable: React.FC<PersonTableProps> = ({
  data,
  fullTreePersons,
  viewSelection,
  onCreate,
  onUpdate,
  onSetPartialView,
  onSetFullView,
}) => {
  const [selectedPersonId, setSelectedPersonId] = useState<string | null>(null);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [visibleColumnIds, setVisibleColumnIds] =
    useState<string[]>(defaultColumnIds);
  const [sortColumnId, setSortColumnId] = useState(NAME_COLUMN_ID);
  const [sortDirection, setSortDirection] =
    useState<SortDirection>("ascending");

  const columns = useMemo<Column[]>(() => {
    const usedInfoKeys = new Set<string>();
    for (const person of fullTreePersons) {
      for (const key of person.info?.keys() ?? []) {
        if (!reservedInfoKeys.has(key)) usedInfoKeys.add(key);
      }
    }
    usedInfoKeys.add("@dateOfBirth");
    usedInfoKeys.add("@dateOfDeath");

    const infoColumns = Array.from(usedInfoKeys)
      .sort((left, right) =>
        infoKeyLabel(left).localeCompare(infoKeyLabel(right), undefined, {
          sensitivity: "base",
        }),
      )
      .map((key) => ({
        id: infoColumnId(key),
        label: infoKeyLabel(key),
        getValue: (person: Person) => person.info?.get(key) ?? "",
      }));

    return [
      {
        id: NAME_COLUMN_ID,
        label: "Name",
        getValue: getPersonName,
      },
      ...infoColumns,
    ];
  }, [fullTreePersons]);

  const visibleColumns = useMemo(
    () => columns.filter((column) => visibleColumnIds.includes(column.id)),
    [columns, visibleColumnIds],
  );
  const effectiveSortColumn =
    visibleColumns.find((column) => column.id === sortColumnId) ??
    visibleColumns[0];

  const sortedPersons = useMemo(() => {
    if (!data || !effectiveSortColumn) return data?.persons ?? [];

    return [...data.persons].sort((left, right) => {
      const leftValue = effectiveSortColumn.getValue(left).trim();
      const rightValue = effectiveSortColumn.getValue(right).trim();

      if (!leftValue && rightValue) return 1;
      if (leftValue && !rightValue) return -1;

      const result = leftValue.localeCompare(rightValue, undefined, {
        numeric: true,
        sensitivity: "base",
      });
      if (result !== 0) {
        return sortDirection === "ascending" ? result : -result;
      }

      const nameResult = getPersonName(left).localeCompare(
        getPersonName(right),
        undefined,
        { sensitivity: "base" },
      );
      return nameResult || left.id.localeCompare(right.id);
    });
  }, [data, effectiveSortColumn, sortDirection]);

  useEffect(() => {
    if (
      selectedPersonId &&
      !fullTreePersons.some((person) => person.id === selectedPersonId)
    ) {
      setSelectedPersonId(null);
      setIsModalOpen(false);
    }
  }, [fullTreePersons, selectedPersonId]);

  if (!data) {
    return (
      <div className="flex h-full w-full flex-col items-center justify-center gap-4">
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
    ? fullTreePersons.find((person) => person.id === selectedPersonId)
    : undefined;
  const viewRoot = viewSelection
    ? fullTreePersons.find((person) => person.id === viewSelection.root)
    : undefined;
  const scopeLabels: Record<TreeViewScope, string> = {
    ancestors: "Ancestors",
    descendants: "Descendants",
    both: "Both",
  };

  const setColumnVisible = (columnId: string, isVisible: boolean) => {
    setVisibleColumnIds((current) => {
      if (isVisible) {
        return columns
          .map((column) => column.id)
          .filter((id) => id === columnId || current.includes(id));
      }
      return current.length === 1
        ? current
        : current.filter((id) => id !== columnId);
    });
  };

  const changeSort = (columnId: string) => {
    if (effectiveSortColumn?.id === columnId) {
      setSortDirection((current) =>
        current === "ascending" ? "descending" : "ascending",
      );
      setSortColumnId(columnId);
      return;
    }
    setSortColumnId(columnId);
    setSortDirection("ascending");
  };

  const openPerson = (personId: string) => {
    setSelectedPersonId(personId);
    setIsModalOpen(true);
  };

  return (
    <div className="flex h-full min-h-0 flex-col bg-background">
      <div className="flex flex-wrap items-start justify-between gap-3 border-b border-border bg-card px-5 py-3">
        <div>
          <p className="text-sm font-medium">People</p>
          <p className="text-xs text-muted-foreground" aria-live="polite">
            {viewSelection
              ? `${scopeLabels[viewSelection.scope]} of ${getPersonName(viewRoot)} · ${data.persons.length} people`
              : selectedPerson
                ? `Choose relatives of ${getPersonName(selectedPerson)} · ${data.persons.length} people`
                : `${data.persons.length} people · Select a person to filter the table`}
          </p>
        </div>

        <div className="flex flex-wrap items-center justify-end gap-2">
          <div
            className="flex gap-1"
            role="group"
            aria-label={
              selectedPerson
                ? `Filter table around ${getPersonName(selectedPerson)}`
                : "Filter table around selected person"
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

          <details className="relative">
            <summary className="flex h-9 cursor-pointer list-none items-center gap-2 rounded-md border border-input bg-background px-3 text-sm font-medium shadow-xs hover:bg-accent hover:text-accent-foreground focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50 [&::-webkit-details-marker]:hidden">
              <Columns3 className="h-4 w-4" aria-hidden="true" />
              Columns
            </summary>
            <fieldset className="absolute right-0 z-20 mt-2 max-h-72 min-w-56 overflow-auto rounded-md border border-border bg-popover p-3 text-popover-foreground shadow-lg">
              <legend className="sr-only">Visible table columns</legend>
              <p className="mb-2 text-xs font-medium text-muted-foreground">
                Visible columns
              </p>
              <div className="space-y-1">
                {columns.map((column) => {
                  const isVisible = visibleColumnIds.includes(column.id);
                  return (
                    <label
                      key={column.id}
                      className="flex cursor-pointer items-center gap-2 rounded px-2 py-1.5 text-sm hover:bg-accent"
                    >
                      <input
                        type="checkbox"
                        checked={isVisible}
                        disabled={isVisible && visibleColumnIds.length === 1}
                        onChange={(event) =>
                          setColumnVisible(column.id, event.target.checked)
                        }
                      />
                      <span>{column.label}</span>
                    </label>
                  );
                })}
              </div>
            </fieldset>
          </details>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-auto">
        <table className="w-full min-w-max border-collapse text-left text-sm">
          <thead className="sticky top-0 z-10 bg-muted/95 backdrop-blur">
            <tr className="border-b border-border">
              <th
                scope="col"
                className="w-16 px-4 py-3 font-medium text-muted-foreground"
              >
                <span className="sr-only">Image</span>
              </th>
              {visibleColumns.map((column) => {
                const isSorted = effectiveSortColumn?.id === column.id;
                return (
                  <th
                    key={column.id}
                    scope="col"
                    aria-sort={isSorted ? sortDirection : "none"}
                    className="min-w-44 px-4 py-3 font-medium"
                  >
                    <button
                      type="button"
                      className="-m-2 flex rounded p-2 hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                      onClick={() => changeSort(column.id)}
                    >
                      <span>{column.label}</span>
                      {isSorted ? (
                        sortDirection === "ascending" ? (
                          <ArrowUp
                            className="ml-2 h-4 w-4"
                            aria-hidden="true"
                          />
                        ) : (
                          <ArrowDown
                            className="ml-2 h-4 w-4"
                            aria-hidden="true"
                          />
                        )
                      ) : (
                        <ArrowUpDown
                          className="ml-2 h-4 w-4 text-muted-foreground"
                          aria-hidden="true"
                        />
                      )}
                      <span className="sr-only">
                        {isSorted && sortDirection === "ascending"
                          ? ", sorted ascending. Activate to sort descending"
                          : isSorted
                            ? ", sorted descending. Activate to sort ascending"
                            : ". Activate to sort ascending"}
                      </span>
                    </button>
                  </th>
                );
              })}
            </tr>
          </thead>
          <tbody>
            {sortedPersons.map((person) => {
              const isSelected = person.id === selectedPersonId;
              return (
                <tr
                  key={person.id}
                  tabIndex={0}
                  aria-selected={isSelected}
                  className={`cursor-pointer border-b border-border transition-colors hover:bg-muted/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring ${
                    isSelected ? "bg-primary/10" : ""
                  }`}
                  onClick={() =>
                    setSelectedPersonId((current) =>
                      current === person.id ? null : person.id,
                    )
                  }
                  onDoubleClick={() => openPerson(person.id)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter") {
                      event.preventDefault();
                      openPerson(person.id);
                    } else if (event.key === " ") {
                      event.preventDefault();
                      setSelectedPersonId((current) =>
                        current === person.id ? null : person.id,
                      );
                    }
                  }}
                >
                  <td className="px-4 py-2">
                    <PersonPreview person={person} />
                  </td>
                  {visibleColumns.map((column) => {
                    const value = column.getValue(person).trim();
                    return (
                      <td
                        key={column.id}
                        className="max-w-80 px-4 py-3 text-card-foreground"
                      >
                        <span
                          className={
                            value ? "block truncate" : "text-muted-foreground"
                          }
                          title={value || undefined}
                        >
                          {value || "—"}
                        </span>
                      </td>
                    );
                  })}
                </tr>
              );
            })}
          </tbody>
        </table>

        {sortedPersons.length === 0 && (
          <div className="flex h-40 items-center justify-center text-sm text-muted-foreground">
            This view contains no people.
          </div>
        )}
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
