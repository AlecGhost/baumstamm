import React, { useEffect, useRef, useState } from "react";
import type { Person, TreeData } from "@/lib/types";
import { getPersonName } from "@/lib/types";
import { convertFileSrc } from "@tauri-apps/api/tauri";
import { WasmServiceLive } from "@/lib/wasm";
import { Effect } from "effect";
import { TreeGrid } from "./TreeGrid";

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
  const canvasRef = useRef<HTMLDivElement>(null);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [zoom, setZoom] = useState(1);
  const [isDragging, setIsDragging] = useState(false);
  const [lastPos, setLastPos] = useState({ x: 0, y: 0 });

  // Wheel zoom scoped to this canvas
  useEffect(() => {
    const container = canvasRef.current;
    if (!container) return;

    const handleWheel = (e: WheelEvent) => {
      e.preventDefault();
      e.stopPropagation();

      const zoomSensitivity = 0.001;
      const delta = -e.deltaY * zoomSensitivity;

      setZoom((prev) => {
        const next = prev * Math.exp(delta);
        return Math.min(Math.max(next, 0.1), 5);
      });
    };

    container.addEventListener("wheel", handleWheel, { passive: false });
    return () => container.removeEventListener("wheel", handleWheel);
  }, []);

  const handlePointerDown = (e: React.PointerEvent) => {
    setIsDragging(true);
    setLastPos({ x: e.clientX, y: e.clientY });
    e.currentTarget.setPointerCapture(e.pointerId);
  };

  const handlePointerMove = (e: React.PointerEvent) => {
    if (!isDragging) return;
    const dx = e.clientX - lastPos.x;
    const dy = e.clientY - lastPos.y;
    setPan((prev) => ({ x: prev.x + dx, y: prev.y + dy }));
    setLastPos({ x: e.clientX, y: e.clientY });
  };

  const handlePointerUp = (e: React.PointerEvent) => {
    setIsDragging(false);
    if (e.currentTarget.hasPointerCapture(e.pointerId)) {
      e.currentTarget.releasePointerCapture(e.pointerId);
    }
  };

  return (
    <div
      ref={canvasRef}
      className="w-full h-80 border rounded-xl overflow-hidden bg-muted/30 relative cursor-grab active:cursor-grabbing select-none"
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      onPointerCancel={handlePointerUp}
    >
      <div
        className="absolute origin-center transition-transform duration-75 ease-out"
        style={{
          transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`,
          left: "50%",
          top: "50%",
          translate: "-50% -50%",
        }}
      >
        <div className="p-8">
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
        show_partner_siblings: false,
        show_ancestor_siblings: false,
        descendent_gen_limit: { Limit: 1 },
        ancestor_gen_limit: { Limit: 1 },
      })
    )
      .then((data) => setSubTreeData(data))
      .catch((err) => console.error("Failed to fetch sub tree data:", err));
  }, [person, isOpen, treeData]); // include treeData to refresh when tree changes

  // Close on Escape key
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && isOpen && !isEditing) {
        onClose();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, onClose, isEditing]);

  useEffect(() => {
    if (!isOpen) {
      setIsEditing(false);
      setNewKeyInput("");
      setNewValueInput("");
    }
  }, [isOpen]);

  useEffect(() => {
    if (isEditing && person && person.info) {
      const initialForm: Record<string, string> = {};
      for (const [key, value] of person.info.entries()) {
        initialForm[key] = value;
      }
      setEditForm(initialForm);
    }
  }, [isEditing, person]);

  if (!isOpen || !person || !person.info) return null;

  const handleSave = async () => {
    if (!person || !person.info) return;
    setIsSaving(true);

    try {
      await Effect.runPromise(
        Effect.gen(function* () {
          // Find removed keys
          for (const [oldKey] of person.info!.entries()) {
            if (!(oldKey in editForm)) {
              yield* WasmServiceLive.removeInfo(person.id, oldKey);
            }
          }
          // Find added/changed keys
          for (const [newKey, newValue] of Object.entries(editForm)) {
            if (person.info!.get(newKey) !== newValue) {
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

  const name = getPersonName(person);

  // When editing, show the raw image path/URL. When viewing, render it.
  const imageValue = isEditing
    ? editForm["@image"] || ""
    : person.info.get("@image");
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
    : person.info.get("@dateOfBirth");
  const dod = isEditing
    ? editForm["@dateOfDeath"]
    : person.info.get("@dateOfDeath");

  const allEntries = isEditing
    ? Object.entries(editForm)
    : Array.from(person.info.entries());
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
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm p-4 animate-in fade-in duration-200"
      onClick={() => {
        if (!isEditing) onClose();
      }}
      onPointerDown={(e) => e.stopPropagation()}
      onPointerUp={(e) => e.stopPropagation()}
    >
      <div
        className="bg-card text-card-foreground border rounded-xl shadow-2xl w-full max-w-lg overflow-hidden flex flex-col max-h-[90vh] animate-in zoom-in-95 duration-200"
        onClick={(e) => e.stopPropagation()}
        onPointerDown={(e) => e.stopPropagation()}
        onPointerUp={(e) => e.stopPropagation()}
      >
        {/* Header/Image Section */}
        <div className="relative shrink-0">
          {!isEditing ? (
            displayImage ? (
              <div className="w-full h-48 bg-muted flex items-center justify-center overflow-hidden">
                <img
                  src={displayImage}
                  alt={name}
                  className="w-full h-full object-cover"
                />
              </div>
            ) : (
              <div className="w-full h-24 bg-primary/10 flex items-center justify-center">
                <span className="text-4xl font-semibold text-primary/40">
                  {name.charAt(0)}
                </span>
              </div>
            )
          ) : (
            <div className="w-full bg-primary/10 flex flex-col p-6 pt-12 pb-4 gap-2 border-b">
              <label className="text-xs font-medium text-muted-foreground">
                Image Path/URL (@image)
              </label>
              <input
                autoFocus
                className="flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                value={editForm["@image"] || ""}
                onChange={(e) =>
                  setEditForm((prev) => ({ ...prev, "@image": e.target.value }))
                }
                placeholder="/path/to/image.jpg or https://..."
              />
            </div>
          )}

          <div className="absolute top-4 right-4 flex gap-2">
            {!isEditing && (
              <button
                onClick={() => setIsEditing(true)}
                className="w-8 h-8 flex items-center justify-center rounded-full bg-black/50 text-white hover:bg-black/70 transition-colors focus:outline-none focus:ring-2 focus:ring-ring"
                aria-label="Edit person"
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
              onClick={() => {
                if (isEditing) setIsEditing(false);
                else onClose();
              }}
              className="w-8 h-8 flex items-center justify-center rounded-full bg-black/50 text-white hover:bg-black/70 transition-colors focus:outline-none focus:ring-2 focus:ring-ring"
              aria-label={isEditing ? "Cancel editing" : "Close modal"}
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
        <div className="p-6 overflow-y-auto">
          {!isEditing ? (
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
                        const displayKey = key.startsWith("@")
                          ? key.substring(1)
                          : key;
                        const formattedKey =
                          displayKey.charAt(0).toUpperCase() +
                          displayKey.slice(1).replace(/([A-Z])/g, " $1");
                        return (
                          <div key={key} className="break-words">
                            <dt className="text-xs text-muted-foreground font-medium mb-0.5">
                              {formattedKey}
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
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <label className="text-xs font-medium text-muted-foreground">
                    First Name (@firstName)
                  </label>
                  <input
                    className="flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
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
                    Last Name (@lastName)
                  </label>
                  <input
                    className="flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
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

              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <label className="text-xs font-medium text-muted-foreground">
                    Date of Birth (@dateOfBirth)
                  </label>
                  <input
                    className="flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
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
                    Date of Death (@dateOfDeath)
                  </label>
                  <input
                    className="flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
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
                        {key}
                      </label>
                      <div className="flex gap-2">
                        <input
                          className="flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                          value={value as string}
                          onChange={(e) =>
                            setEditForm((prev) => ({
                              ...prev,
                              [key]: e.target.value,
                            }))
                          }
                        />
                        <button
                          className="flex-shrink-0 h-9 px-3 text-destructive hover:bg-destructive/10 rounded-md transition-colors text-sm font-medium"
                          onClick={() =>
                            setEditForm((prev) => {
                              const next = { ...prev };
                              delete next[key];
                              return next;
                            })
                          }
                          title={`Remove ${key}`}
                        >
                          Remove
                        </button>
                      </div>
                    </div>
                  </div>
                ))}

                <div className="flex gap-2 items-end pt-2">
                  <div className="flex-1 space-y-1">
                    <label className="text-xs font-medium text-muted-foreground">
                      New Key
                    </label>
                    <input
                      className="flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                      value={newKeyInput}
                      onChange={(e) => setNewKeyInput(e.target.value)}
                      placeholder="e.g. occupation"
                    />
                  </div>
                  <div className="flex-1 space-y-1">
                    <label className="text-xs font-medium text-muted-foreground">
                      Value
                    </label>
                    <input
                      className="flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                      value={newValueInput}
                      onChange={(e) => setNewValueInput(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") handleAddNewField();
                      }}
                      placeholder="Value"
                    />
                  </div>
                  <button
                    className="h-9 px-4 bg-secondary text-secondary-foreground hover:bg-secondary/80 rounded-md transition-colors text-sm font-medium bg-muted"
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
          <div className="p-4 border-t bg-muted/20 flex justify-end gap-2 shrink-0">
            <button
              onClick={() => setIsEditing(false)}
              className="h-9 px-4 text-sm font-medium rounded-md hover:bg-accent hover:text-accent-foreground transition-colors disabled:opacity-50"
              disabled={isSaving}
            >
              Cancel
            </button>
            <button
              onClick={handleSave}
              className="h-9 px-4 text-sm font-medium rounded-md bg-primary text-primary-foreground text-white hover:bg-primary/90 transition-colors disabled:opacity-50 flex items-center gap-2"
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
