import React, { useEffect } from "react";
import type { Person } from "@/lib/types";
import { getPersonName } from "@/lib/types";
import { convertFileSrc } from "@tauri-apps/api/tauri";

interface PersonDetailsModalProps {
  person: Person | null;
  isOpen: boolean;
  onClose: () => void;
}

export const PersonDetailsModal: React.FC<PersonDetailsModalProps> = ({
  person,
  isOpen,
  onClose,
}) => {
  // Close on Escape key
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && isOpen) {
        onClose();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, onClose]);

  if (!isOpen || !person || !person.info) return null;

  const name = getPersonName(person);

  let image = person.info.get("@image");
  if (image) {
    // If it's a local absolute path and we are running in Tauri, use asset protocol.
    // Check for Tauri environment dynamically (or by availability of tauri API)
    if (image.startsWith("/")) {
      if (window.__TAURI__) {
        try {
          image = convertFileSrc(image);
        } catch (e) {
          console.warn("Failed to convert image source:", e);
        }
      } else {
        // We are in the browser, local absolute paths will not resolve. Show placeholder.
        image = undefined;
      }
    }
  }

  const dob = person.info.get("@dateOfBirth");
  const dod = person.info.get("@dateOfDeath");

  // Get remaining entries
  const allEntries = Array.from(person.info.entries());
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
      onClick={onClose}
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
        <div className="relative">
          {image ? (
            <div className="w-full h-48 bg-muted flex items-center justify-center overflow-hidden">
              <img
                src={image}
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
          )}
          <button
            onClick={onClose}
            className="absolute top-4 right-4 w-8 h-8 flex items-center justify-center rounded-full bg-black/50 text-white hover:bg-black/70 transition-colors focus:outline-none focus:ring-2 focus:ring-ring"
            aria-label="Close modal"
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

        {/* Content Section */}
        <div className="p-6 overflow-y-auto">
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
                        <dd className="text-sm">{value}</dd>
                      </div>
                    );
                  })}
                </dl>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
