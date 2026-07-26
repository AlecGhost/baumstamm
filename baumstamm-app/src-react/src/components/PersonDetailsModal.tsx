import React, { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { Person, Relationship, TreeData } from "@/lib/types";
import { getPersonName } from "@/lib/types";
import { convertFileSrc } from "@tauri-apps/api/tauri";
import { WasmServiceLive } from "@/lib/wasm";
import { Effect } from "effect";
import { TreeGrid } from "./TreeGrid";
import { usePanZoom } from "@/hooks/use-pan-zoom";
import { getInfoKeyLabel } from "@/lib/utils";

// ---------------------------------------------------------------------------
// Embedded pan/zoom canvas for the sub-tree inside the modal
// ---------------------------------------------------------------------------
interface EmbeddedTreeCanvasProps {
  data: TreeData;
  selectedPersonId: string | null;
  onSelectPerson: (id: string) => void;
}

const EmbeddedTreeCanvas: React.FC<EmbeddedTreeCanvasProps> = ({
  data,
  selectedPersonId,
  onSelectPerson,
}) => {
  const { containerRef, setZoom, setPan, pointerHandlers, transformStyle } =
    usePanZoom({
      enabled: true,
    });
  const contentRef = useRef<HTMLDivElement>(null);

  // After render, measure content vs container and zoom to fit
  useLayoutEffect(() => {
    const container = containerRef.current;
    const content = contentRef.current;
    if (!container || !content) return;

    const cw = container.clientWidth;
    const ch = container.clientHeight;
    const sw = content.scrollWidth;
    const sh = content.scrollHeight;

    setPan({ x: 0, y: 0 });
    if (sw > 0 && sh > 0) {
      const fitZoom = Math.min(cw / sw, ch / sh, 1);
      setZoom(fitZoom);
    }
  }, [data, setZoom, setPan, containerRef]);

  return (
    <div
      ref={containerRef}
      className="relative h-64 w-full cursor-grab select-none overflow-hidden rounded-xl border bg-muted/30 active:cursor-grabbing sm:h-80"
      {...pointerHandlers}
    >
      <div
        className="absolute origin-center transition-transform duration-75 ease-out"
        style={transformStyle}
      >
        <div ref={contentRef} className="p-4 sm:p-8">
          <TreeGrid
            data={data}
            selectedPersonId={selectedPersonId}
            onSelectPerson={onSelectPerson}
            onDoubleClickPerson={() => {}}
          />
        </div>
      </div>
    </div>
  );
};

// ---------------------------------------------------------------------------

interface PersonDetailsModalProps {
  person: Person | null;
  treeData?: TreeData | null;
  isOpen: boolean;
  onClose: () => void;
  onUpdate: () => void;
  onSelectPerson: (id: string) => void;
}

export const PersonDetailsModal: React.FC<PersonDetailsModalProps> = ({
  person,
  treeData,
  isOpen,
  onClose,
  onUpdate,
  onSelectPerson,
}) => {
  const [isEditing, setIsEditing] = useState(false);
  const [isActionView, setIsActionView] = useState(false);
  const [actionState, setActionState] = useState<
    { type: "none" } | { type: "partner" } | { type: "merge" }
  >({ type: "none" });
  const [actionSearchQuery, setActionSearchQuery] = useState("");
  const [editForm, setEditForm] = useState<Record<string, string>>({});
  const [isSaving, setIsSaving] = useState(false);
  const [newKeyInput, setNewKeyInput] = useState("");
  const [newValueInput, setNewValueInput] = useState("");

  const [subTreeData, setSubTreeData] = useState<TreeData | null>(null);

  useEffect(() => {
    if (!person || !isOpen) return;

    Effect.runPromise(
      WasmServiceLive.getSubTreeData(person.id, {
        show_partners: true,
        show_siblings: true,
        show_partner_siblings: false,
        show_ancestor_siblings: false,
        descendent_gen_limit: { Limit: 1 },
        ancestor_gen_limit: { Limit: 1 },
      }),
    )
      .then((data) => setSubTreeData(data))
      .catch((err) => console.error("Failed to fetch sub tree data:", err));
  }, [person, isOpen, treeData]); // include treeData to refresh when tree changes

  const handleCloseOrBack = React.useCallback(() => {
    if (isEditing) {
      setIsEditing(false);
    } else if (isActionView) {
      setIsActionView(false);
      setActionState({ type: "none" });
      setActionSearchQuery("");
    } else {
      onClose();
    }
  }, [isEditing, isActionView, onClose]);

  // Handle keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!isOpen) return;

      if (e.key === "Escape") {
        e.preventDefault();
        handleCloseOrBack();
        return;
      }

      // Letter shortcuts should not interrupt text entry or modified commands.
      const target = e.target;
      if (
        e.altKey ||
        e.ctrlKey ||
        e.metaKey ||
        e.shiftKey ||
        (target instanceof HTMLElement &&
          (["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName) ||
            target.isContentEditable ||
            target.closest('[contenteditable="true"]')))
      ) {
        return;
      }

      if (!isEditing && !isActionView) {
        if (e.key === "a" || e.key === "A") {
          e.preventDefault();
          setIsActionView(true);
        } else if (e.key === "e" || e.key === "E") {
          e.preventDefault();
          setIsEditing(true);
        }
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, handleCloseOrBack, isEditing, isActionView]);

  useEffect(() => {
    if (!isOpen) {
      setIsEditing(false);
      setIsActionView(false);
      setActionState({ type: "none" });
      setActionSearchQuery("");
      setNewKeyInput("");
      setNewValueInput("");
    }
  }, [isOpen]);

  useEffect(() => {
    if (isEditing && person) {
      const initialForm: Record<string, string> = {};
      if (person.info) {
        for (const [key, value] of person.info.entries()) {
          initialForm[key] = value;
        }
      }
      setEditForm(initialForm);
    }
  }, [isEditing, person]);

  if (!isOpen || !person) return null;

  const handleSave = async () => {
    if (!person) return;
    setIsSaving(true);

    try {
      await Effect.runPromise(
        Effect.gen(function* () {
          const currentInfo = person.info || new Map<string, string>();
          // Find removed keys
          for (const [oldKey] of currentInfo.entries()) {
            if (!(oldKey in editForm)) {
              yield* WasmServiceLive.removeInfo(person.id, oldKey);
            }
          }
          // Find added/changed keys
          for (const [newKey, newValue] of Object.entries(editForm)) {
            if (currentInfo.get(newKey) !== newValue) {
              yield* WasmServiceLive.insertInfo(person.id, newKey, newValue);
            }
          }
        }),
      );

      setIsEditing(false);
      onUpdate();
    } catch (err) {
      console.error("Failed to save person details:", err);
    } finally {
      setIsSaving(false);
    }
  };

  const handleAddNewField = () => {
    if (newKeyInput.trim() && newValueInput.trim()) {
      const key = newKeyInput.trim();
      setEditForm((prev) => ({ ...prev, [key]: newValueInput.trim() }));
      setNewKeyInput("");
      setNewValueInput("");
    }
  };

  const handleAddParent = async () => {
    if (!person || !treeData) return;
    const rel = treeData.relationships.find((r) =>
      r.children.includes(person.id),
    );
    if (rel) {
      try {
        await Effect.runPromise(WasmServiceLive.addParent(rel.id));
        onUpdate();
        setIsActionView(false);
      } catch (e) {
        console.error("Failed to add parent:", e);
      }
    }
  };

  const handleAddChild = async (specificRelId?: string) => {
    if (!person || !treeData) return;
    const rels = treeData.relationships.filter((r) =>
      r.parents.includes(person.id),
    );
    try {
      if (specificRelId) {
        await Effect.runPromise(WasmServiceLive.addChild(specificRelId));
      } else if (rels.length === 0) {
        const newRelId = await Effect.runPromise(
          WasmServiceLive.addNewRelationship(person.id),
        );
        await Effect.runPromise(WasmServiceLive.addChild(newRelId));
      } else if (rels.length === 1) {
        await Effect.runPromise(WasmServiceLive.addChild(rels[0].id));
      } else {
        await Effect.runPromise(WasmServiceLive.addChild(rels[0].id));
      }
      onUpdate();
      setIsActionView(false);
    } catch (e) {
      console.error("Failed to add child:", e);
    }
  };

  const handleAddNewPartner = async () => {
    if (!person || !treeData) return;
    try {
      const newRelId = await Effect.runPromise(
        WasmServiceLive.addNewRelationship(person.id),
      );
      await Effect.runPromise(WasmServiceLive.addParent(newRelId));
      onUpdate();
      setIsActionView(false);
    } catch (e) {
      console.error("Failed to add new partner:", e);
    }
  };

  const handleAddPartner = async (partnerId: string) => {
    if (!person) return;
    try {
      await Effect.runPromise(
        WasmServiceLive.addRelationshipWithPartner(person.id, partnerId),
      );
      onUpdate();
      setIsActionView(false);
      setActionState({ type: "none" });
    } catch (e) {
      console.error("Failed to add partner:", e);
    }
  };

  const handleMergePerson = async (otherId: string) => {
    if (!person) return;
    try {
      await Effect.runPromise(WasmServiceLive.mergePerson(person.id, otherId));
      onUpdate();
      setIsActionView(false);
      setActionState({ type: "none" });
    } catch (e) {
      console.error("Failed to merge person:", e);
    }
  };

  const handleRemovePerson = async () => {
    if (!person) return;
    try {
      await Effect.runPromise(WasmServiceLive.removePerson(person.id));
      onUpdate();
      onClose();
    } catch (e) {
      console.error("Failed to remove person:", e);
    }
  };

  const getPartnerName = (rel: Relationship, currentPersonId: string) => {
    if (!treeData) return "Unknown";
    const partnerId = rel.parents.find(
      (p: string | null) => p !== null && p !== currentPersonId,
    );
    if (!partnerId) return "Unknown";
    const partner = treeData.persons.find((p) => p.id === partnerId);
    return getPersonName(partner);
  };

  const parentRels =
    treeData && person
      ? treeData.relationships.filter((r) => r.parents.includes(person.id))
      : [];

  const name = getPersonName(person);

  // When editing, show the raw image path/URL. When viewing, render it.
  const imageValue = isEditing
    ? editForm["@image"] || ""
    : person.info?.get("@image");
  let displayImage = imageValue;
  if (!isEditing && displayImage && displayImage.startsWith("/")) {
    if (typeof window !== "undefined" && window.__TAURI__) {
      try {
        displayImage = convertFileSrc(displayImage);
      } catch (e) {
        console.warn("Failed to convert image source:", e);
      }
    } else {
      displayImage = undefined;
    }
  }

  // If viewing, use person.info. If editing, use editForm.
  const dob = isEditing
    ? editForm["@dateOfBirth"]
    : person.info?.get("@dateOfBirth");
  const dod = isEditing
    ? editForm["@dateOfDeath"]
    : person.info?.get("@dateOfDeath");

  const allEntries = isEditing
    ? Object.entries(editForm)
    : person.info
      ? Array.from(person.info.entries())
      : [];
  const excludeKeys = [
    "@image",
    "@dateOfBirth",
    "@dateOfDeath",
    "@firstName",
    "@lastName",
  ];
  const remainingEntries = allEntries.filter(
    ([key]) => !excludeKeys.includes(key),
  );

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-0 backdrop-blur-sm animate-in fade-in duration-200 sm:p-4"
      onClick={() => {
        if (!isEditing) onClose();
      }}
      onPointerDown={(e) => e.stopPropagation()}
      onPointerUp={(e) => e.stopPropagation()}
    >
      <div
        className="flex h-dvh max-h-dvh w-full max-w-lg flex-col overflow-hidden border bg-card text-card-foreground shadow-2xl animate-in zoom-in-95 duration-200 sm:h-auto sm:max-h-[90vh] sm:rounded-xl"
        onClick={(e) => e.stopPropagation()}
        onPointerDown={(e) => e.stopPropagation()}
        onPointerUp={(e) => e.stopPropagation()}
      >
        {/* Header/Image Section */}
        <div className="relative shrink-0">
          {!isEditing && !isActionView ? (
            displayImage ? (
              <div className="flex h-36 w-full items-center justify-center overflow-hidden bg-muted sm:h-48">
                <img
                  src={displayImage}
                  alt={name}
                  className="h-full w-full bg-muted object-contain"
                />
              </div>
            ) : (
              <div className="flex h-20 w-full items-center justify-center bg-primary/10 sm:h-24">
                <span className="text-4xl font-semibold text-primary/40">
                  {name.charAt(0)}
                </span>
              </div>
            )
          ) : (
            <div className="flex w-full flex-col gap-2 border-b bg-primary/10 p-4 pb-3 pt-16 sm:p-6 sm:pb-4 sm:pt-12">
              {isActionView ? (
                <div className="flex items-center">
                  <h2 className="text-xl font-bold tracking-tight">Actions</h2>
                </div>
              ) : (
                <>
                  <label className="text-xs font-medium text-muted-foreground">
                    Image path or URL
                  </label>
                  <input
                    autoFocus
                    className="flex h-11 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 sm:h-9"
                    value={editForm["@image"] || ""}
                    onChange={(e) =>
                      setEditForm((prev) => ({
                        ...prev,
                        "@image": e.target.value,
                      }))
                    }
                    placeholder="/path/to/image.jpg or https://..."
                  />
                </>
              )}
            </div>
          )}

          <div className="absolute right-3 top-3 flex gap-2 sm:right-4 sm:top-4">
            {!isEditing && !isActionView && (
              <button
                onClick={() => setIsActionView(true)}
                className="flex h-11 w-11 touch-manipulation items-center justify-center rounded-full bg-black/50 text-white transition-colors hover:bg-black/70 focus:outline-none focus:ring-2 focus:ring-ring sm:h-8 sm:w-8"
                aria-label="Actions"
                aria-keyshortcuts="A"
                title="Actions (A)"
              >
                <svg
                  width="15"
                  height="15"
                  viewBox="0 0 15 15"
                  fill="none"
                  xmlns="http://www.w3.org/2000/svg"
                >
                  <path
                    d="M7.49991 1V7.5M7.49991 14V7.5M7.49991 7.5H14M7.49991 7.5H1"
                    stroke="currentColor"
                    strokeLinecap="round"
                    strokeWidth="1.5"
                  />
                </svg>
              </button>
            )}
            {!isEditing && !isActionView && (
              <button
                onClick={() => setIsEditing(true)}
                className="flex h-11 w-11 touch-manipulation items-center justify-center rounded-full bg-black/50 text-white transition-colors hover:bg-black/70 focus:outline-none focus:ring-2 focus:ring-ring sm:h-8 sm:w-8"
                aria-label="Edit person"
                aria-keyshortcuts="E"
                title="Edit person (E)"
              >
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 15 15"
                  fill="none"
                  xmlns="http://www.w3.org/2000/svg"
                >
                  <path
                    d="M11.8536 1.14645C11.6583 0.951184 11.3417 0.951184 11.1465 1.14645L3.71455 8.57836C3.62459 8.66832 3.55263 8.77461 3.50251 8.89155L2.04044 12.303C1.9599 12.491 2.00189 12.709 2.14646 12.8536C2.29103 12.9981 2.50905 13.0401 2.69697 12.9596L6.10847 11.4975C6.2254 11.4474 6.3317 11.3754 6.42166 11.2855L13.8536 3.85355C14.0488 3.65829 14.0488 3.34171 13.8536 3.14645L11.8536 1.14645ZM4.42166 9.20711L10.5 3.12876L11.8712 4.5L5.79289 10.5784L4.42166 9.20711ZM2.45321 12.5468L3.39151 10.3582L4.64175 11.6085L2.45321 12.5468Z"
                    fill="currentColor"
                    fillRule="evenodd"
                    clipRule="evenodd"
                  ></path>
                </svg>
              </button>
            )}
            <button
              onClick={handleCloseOrBack}
              className="flex h-11 w-11 touch-manipulation items-center justify-center rounded-full bg-black/50 text-white transition-colors hover:bg-black/70 focus:outline-none focus:ring-2 focus:ring-ring sm:h-8 sm:w-8"
              aria-label={isEditing || isActionView ? "Cancel" : "Close modal"}
              aria-keyshortcuts="Escape"
              title={
                isEditing || isActionView ? "Back (Escape)" : "Close (Escape)"
              }
            >
              <svg
                width="15"
                height="15"
                viewBox="0 0 15 15"
                fill="none"
                xmlns="http://www.w3.org/2000/svg"
              >
                <path
                  d="M12.8536 2.85355C13.0488 2.65829 13.0488 2.34171 12.8536 2.14645C12.6583 1.95118 12.3417 1.95118 12.1464 2.14645L7.5 6.79289L2.85355 2.14645C2.65829 1.95118 2.34171 1.95118 2.14645 2.14645C1.95118 2.34171 1.95118 2.65829 2.14645 2.85355L6.79289 7.5L2.14645 12.1464C1.95118 12.3417 1.95118 12.6583 2.14645 12.8536C2.34171 13.0488 2.65829 13.0488 2.85355 12.8536L7.5 8.20711L12.1464 12.8536C12.3417 13.0488 12.6583 13.0488 12.8536 12.8536C13.0488 12.6583 13.0488 12.3417 12.8536 12.1464L8.20711 7.5L12.8536 2.85355Z"
                  fill="currentColor"
                  fillRule="evenodd"
                  clipRule="evenodd"
                ></path>
              </svg>
            </button>
          </div>
        </div>

        {/* Content Section */}
        <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain p-4 sm:p-6">
          {isActionView ? (
            <div className="space-y-4">
              {actionState.type === "none" ? (
                <div className="flex flex-col gap-2">
                  <button
                    onClick={handleAddParent}
                    className="min-h-11 w-full rounded-md border bg-muted px-4 py-3 text-left text-sm font-medium transition-colors hover:bg-muted/80"
                  >
                    Add Parent
                  </button>
                  {parentRels.length <= 1 ? (
                    <button
                      onClick={() => handleAddChild()}
                      className="min-h-11 w-full rounded-md border bg-muted px-4 py-3 text-left text-sm font-medium transition-colors hover:bg-muted/80"
                    >
                      Add Child
                    </button>
                  ) : (
                    parentRels.map((rel) => (
                      <button
                        key={rel.id}
                        onClick={() => handleAddChild(rel.id)}
                        className="min-h-11 w-full rounded-md border bg-muted px-4 py-3 text-left text-sm font-medium transition-colors hover:bg-muted/80"
                      >
                        Add Child (with {getPartnerName(rel, person.id)})
                      </button>
                    ))
                  )}
                  <button
                    onClick={handleAddNewPartner}
                    className="min-h-11 w-full rounded-md border bg-muted px-4 py-3 text-left text-sm font-medium transition-colors hover:bg-muted/80"
                  >
                    Add New Partner
                  </button>
                  <button
                    onClick={() => setActionState({ type: "partner" })}
                    className="min-h-11 w-full rounded-md border bg-muted px-4 py-3 text-left text-sm font-medium transition-colors hover:bg-muted/80"
                  >
                    Add Existing Partner...
                  </button>
                  <button
                    onClick={() => setActionState({ type: "merge" })}
                    className="min-h-11 w-full rounded-md border bg-muted px-4 py-3 text-left text-sm font-medium transition-colors hover:bg-muted/80"
                  >
                    Merge Person...
                  </button>
                  <div className="pt-4 border-t mt-4">
                    <button
                      onClick={handleRemovePerson}
                      className="min-h-11 w-full rounded-md border border-destructive/20 bg-destructive/10 px-4 py-3 text-left text-sm font-medium text-destructive transition-colors hover:bg-destructive/20"
                    >
                      Remove Person
                    </button>
                  </div>
                </div>
              ) : (
                <div className="space-y-4">
                  <button
                    onClick={() => {
                      setActionState({ type: "none" });
                      setActionSearchQuery("");
                    }}
                    className="flex min-h-11 items-center gap-1 text-xs text-muted-foreground underline hover:text-foreground"
                  >
                    <svg
                      width="12"
                      height="12"
                      viewBox="0 0 15 15"
                      fill="none"
                      xmlns="http://www.w3.org/2000/svg"
                    >
                      <path
                        d="M6.85355 3.14645C7.04882 3.34171 7.04882 3.65829 6.85355 3.85355L3.70711 7H12.5C12.7761 7 13 7.22386 13 7.5C13 7.77614 12.7761 8 12.5 8H3.70711L6.85355 11.1464C7.04882 11.3417 7.04882 11.6583 6.85355 11.8536C6.65829 12.0488 6.34171 12.0488 6.14645 11.8536L2.14645 7.85355C1.95118 7.65829 1.95118 7.34171 2.14645 7.14645L6.14645 3.14645C6.34171 2.95118 6.65829 2.95118 6.85355 3.14645Z"
                        fill="currentColor"
                        fillRule="evenodd"
                        clipRule="evenodd"
                      ></path>
                    </svg>
                    Back to Actions
                  </button>
                  <h3 className="text-sm font-semibold">
                    {actionState.type === "partner"
                      ? "Select Partner"
                      : "Select Person to Merge"}
                  </h3>
                  <input
                    type="text"
                    placeholder="Search by name..."
                    autoFocus
                    className="flex h-11 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring sm:h-9"
                    value={actionSearchQuery}
                    onChange={(e) => setActionSearchQuery(e.target.value)}
                  />
                  <div className="max-h-60 overflow-y-auto border rounded-md divide-y">
                    {treeData?.persons
                      .filter((p) => p.id !== person.id)
                      .filter((p) =>
                        getPersonName(p)
                          .toLowerCase()
                          .includes(actionSearchQuery.toLowerCase()),
                      )
                      .map((p) => (
                        <button
                          key={p.id}
                          onClick={() => {
                            if (actionState.type === "partner")
                              handleAddPartner(p.id);
                            else if (actionState.type === "merge")
                              handleMergePerson(p.id);
                          }}
                          className="min-h-11 w-full px-3 py-2 text-left text-sm transition-colors hover:bg-muted"
                        >
                          {getPersonName(p)}
                        </button>
                      ))}
                    {treeData?.persons.filter((p) => p.id !== person.id)
                      .length === 0 && (
                      <div className="px-3 py-2 text-sm text-muted-foreground">
                        No other persons found.
                      </div>
                    )}
                  </div>
                </div>
              )}
            </div>
          ) : !isEditing ? (
            <>
              <h2 className="text-2xl font-bold font-sans tracking-tight mb-4">
                {name}
              </h2>

              <div className="space-y-4">
                {(dob || dod) && (
                  <div className="flex flex-wrap gap-4 bg-muted/50 p-3 rounded-lg">
                    {dob && (
                      <div>
                        <span className="text-xs font-medium text-muted-foreground uppercase tracking-wider block mb-1">
                          Born
                        </span>
                        <span className="font-medium">{dob}</span>
                      </div>
                    )}
                    {dod && (
                      <div>
                        <span className="text-xs font-medium text-muted-foreground uppercase tracking-wider block mb-1">
                          Died
                        </span>
                        <span className="font-medium">{dod}</span>
                      </div>
                    )}
                  </div>
                )}

                {/* Sub Tree */}
                {subTreeData && subTreeData.persons.length > 1 && (
                  <div className="space-y-3 pt-2">
                    <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-2">
                      Immediate Family
                    </h3>
                    <EmbeddedTreeCanvas
                      data={subTreeData}
                      selectedPersonId={person?.id ?? null}
                      onSelectPerson={onSelectPerson}
                    />
                  </div>
                )}

                {remainingEntries.length > 0 && (
                  <div className="pt-2">
                    <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wider mb-3">
                      Additional Details
                    </h3>
                    <dl className="grid grid-cols-1 sm:grid-cols-2 gap-x-4 gap-y-3">
                      {remainingEntries.map(([key, value]) => {
                        return (
                          <div key={key} className="break-words">
                            <dt className="text-xs text-muted-foreground font-medium mb-0.5">
                              {getInfoKeyLabel(key)}
                            </dt>
                            <dd className="text-sm">
                              {value as React.ReactNode}
                            </dd>
                          </div>
                        );
                      })}
                    </dl>
                  </div>
                )}
              </div>
            </>
          ) : (
            <div className="space-y-6">
              <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
                <div className="space-y-2">
                  <label className="text-xs font-medium text-muted-foreground">
                    First name
                  </label>
                  <input
                    className="flex h-11 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring sm:h-9"
                    value={editForm["@firstName"] || ""}
                    onChange={(e) =>
                      setEditForm((prev) => ({
                        ...prev,
                        "@firstName": e.target.value,
                      }))
                    }
                  />
                </div>
                <div className="space-y-2">
                  <label className="text-xs font-medium text-muted-foreground">
                    Last name
                  </label>
                  <input
                    className="flex h-11 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring sm:h-9"
                    value={editForm["@lastName"] || ""}
                    onChange={(e) =>
                      setEditForm((prev) => ({
                        ...prev,
                        "@lastName": e.target.value,
                      }))
                    }
                  />
                </div>
              </div>

              <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
                <div className="space-y-2">
                  <label className="text-xs font-medium text-muted-foreground">
                    Date of birth
                  </label>
                  <input
                    className="flex h-11 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring sm:h-9"
                    value={editForm["@dateOfBirth"] || ""}
                    onChange={(e) =>
                      setEditForm((prev) => ({
                        ...prev,
                        "@dateOfBirth": e.target.value,
                      }))
                    }
                    placeholder="YYYY-MM-DD"
                  />
                </div>
                <div className="space-y-2">
                  <label className="text-xs font-medium text-muted-foreground">
                    Date of death
                  </label>
                  <input
                    className="flex h-11 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring sm:h-9"
                    value={editForm["@dateOfDeath"] || ""}
                    onChange={(e) =>
                      setEditForm((prev) => ({
                        ...prev,
                        "@dateOfDeath": e.target.value,
                      }))
                    }
                    placeholder="YYYY-MM-DD"
                  />
                </div>
              </div>

              <div className="space-y-4 pt-4 border-t">
                <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wider">
                  Additional Details
                </h3>

                {remainingEntries.map(([key, value]) => (
                  <div key={key} className="flex gap-2 items-start">
                    <div className="flex-1 space-y-1">
                      <label className="text-xs font-medium text-muted-foreground">
                        {getInfoKeyLabel(key)}
                      </label>
                      <div className="flex flex-col gap-2 sm:flex-row">
                        <input
                          className="flex h-11 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring sm:h-9"
                          value={value as string}
                          onChange={(e) =>
                            setEditForm((prev) => ({
                              ...prev,
                              [key]: e.target.value,
                            }))
                          }
                        />
                        <button
                          className="h-11 flex-shrink-0 rounded-md px-3 text-sm font-medium text-destructive transition-colors hover:bg-destructive/10 sm:h-9"
                          onClick={() =>
                            setEditForm((prev) => {
                              const next = { ...prev };
                              delete next[key];
                              return next;
                            })
                          }
                          title={`Remove ${getInfoKeyLabel(key)}`}
                        >
                          Remove
                        </button>
                      </div>
                    </div>
                  </div>
                ))}

                <div className="flex flex-col gap-3 pt-2 sm:flex-row sm:items-end sm:gap-2">
                  <div className="flex-1 space-y-1">
                    <label className="text-xs font-medium text-muted-foreground">
                      Field name
                    </label>
                    <input
                      className="flex h-11 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring sm:h-9"
                      value={newKeyInput}
                      onChange={(e) => setNewKeyInput(e.target.value)}
                      placeholder="e.g. occupation"
                    />
                  </div>
                  <div className="flex-1 space-y-1">
                    <label className="text-xs font-medium text-muted-foreground">
                      Field value
                    </label>
                    <input
                      className="flex h-11 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring sm:h-9"
                      value={newValueInput}
                      onChange={(e) => setNewValueInput(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") handleAddNewField();
                      }}
                      placeholder="Value"
                    />
                  </div>
                  <button
                    className="h-11 rounded-md bg-muted px-4 text-sm font-medium text-secondary-foreground transition-colors hover:bg-secondary/80 sm:h-9"
                    onClick={handleAddNewField}
                    disabled={!newKeyInput.trim() || !newValueInput.trim()}
                  >
                    Add
                  </button>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        {isEditing && (
          <div className="flex shrink-0 gap-2 border-t bg-muted/20 p-3 pb-[max(0.75rem,env(safe-area-inset-bottom))] sm:justify-end sm:p-4">
            <button
              onClick={() => setIsEditing(false)}
              className="h-11 flex-1 rounded-md px-4 text-sm font-medium transition-colors hover:bg-accent hover:text-accent-foreground disabled:opacity-50 sm:h-9 sm:flex-none"
              disabled={isSaving}
            >
              Cancel
            </button>
            <button
              onClick={handleSave}
              className="flex h-11 flex-1 items-center justify-center gap-2 rounded-md bg-primary px-4 text-sm font-medium text-white text-primary-foreground transition-colors hover:bg-primary/90 disabled:opacity-50 sm:h-9 sm:flex-none"
              disabled={isSaving}
            >
              {isSaving && (
                <div className="w-3 h-3 border-2 border-primary-foreground border-t-transparent rounded-full animate-spin" />
              )}
              Save Changes
            </button>
          </div>
        )}
      </div>
    </div>
  );
};
