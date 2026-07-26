import { useCallback, useEffect, useRef, useState } from "react";
import { Effect } from "effect";
import { save as showSaveDialog } from "@tauri-apps/api/dialog";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/tauri";
import {
  Check,
  Download,
  LoaderCircle,
  Network,
  Save,
  TableProperties,
} from "lucide-react";
import { WasmServiceLive } from "@/lib/wasm";
import type {
  Person,
  TreeData,
  TreeViewScope,
  TreeViewSelection,
  ViewOptions,
} from "@/lib/types";
import {
  LoadTreeDialog,
  type LoadTreeDialogHandle,
} from "@/components/LoadTreeDialog";
import { PersonTable } from "@/components/PersonTable";
import { TreeCanvas } from "@/components/TreeCanvas";
import { Button } from "@/components/ui/button";

type OpenTreePayload = {
  path: string;
  content: string;
};

type SaveStatus = "idle" | "saving" | "saved";
type MainView = "tree" | "table";

const viewOptionsForScope = (scope: TreeViewScope): ViewOptions => ({
  show_partners: true,
  show_siblings: false,
  show_partner_siblings: false,
  show_ancestor_siblings: false,
  ancestor_gen_limit: scope === "descendants" ? { Limit: 0 } : "Unlimited",
  descendent_gen_limit: scope === "ancestors" ? { Limit: 0 } : "Unlimited",
});

const isTauri = () => typeof window !== "undefined" && "__TAURI__" in window;

const fileNameFromPath = (path: string) =>
  path.split(/[\\/]/).pop() || "family-tree.json";

const withJsonExtension = (name: string) =>
  name.toLowerCase().endsWith(".json") ? name : `${name}.json`;

const downloadTree = (content: string, fileName: string) => {
  const blob = new Blob([content], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = withJsonExtension(fileName);
  document.body.appendChild(link);
  link.click();
  link.remove();
  window.setTimeout(() => URL.revokeObjectURL(url), 0);
};

function App() {
  const [isWasmLoaded, setIsWasmLoaded] = useState<boolean>(false);
  const [treeData, setTreeData] = useState<TreeData | null>(null);
  const [fullTreePersons, setFullTreePersons] = useState<Person[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [treeKey, setTreeKey] = useState<number>(0);
  const [treeViewSelection, setTreeViewSelection] =
    useState<TreeViewSelection | null>(null);
  const [saveStatus, setSaveStatus] = useState<SaveStatus>("idle");
  const [mainView, setMainView] = useState<MainView>("tree");
  const fileNameRef = useRef<string>("family-tree.json");
  const currentPathRef = useRef<string | null>(null);
  const hasTreeRef = useRef<boolean>(false);
  const savingRef = useRef<boolean>(false);
  const savedResetTimeoutRef = useRef<number | null>(null);
  const loadTreeDialogRef = useRef<LoadTreeDialogHandle>(null);

  // Initialise WASM on mount
  useEffect(() => {
    Effect.runPromise(
      Effect.gen(function* () {
        yield* WasmServiceLive.init;
        setIsWasmLoaded(true);
      }),
    ).catch((err) => {
      console.error(err);
      setError("Failed to initialize WASM library.");
    });
  }, []);

  const resetSaveStatus = useCallback(() => {
    if (savedResetTimeoutRef.current !== null) {
      window.clearTimeout(savedResetTimeoutRef.current);
      savedResetTimeoutRef.current = null;
    }
    setSaveStatus("idle");
  }, []);

  const showSavedStatus = useCallback(() => {
    if (savedResetTimeoutRef.current !== null) {
      window.clearTimeout(savedResetTimeoutRef.current);
    }
    setSaveStatus("saved");
    savedResetTimeoutRef.current = window.setTimeout(() => {
      setSaveStatus("idle");
      savedResetTimeoutRef.current = null;
    }, 2000);
  }, []);

  useEffect(
    () => () => {
      if (savedResetTimeoutRef.current !== null) {
        window.clearTimeout(savedResetTimeoutRef.current);
      }
    },
    [],
  );

  const handleLoadTree = useCallback(
    (
      fileContent: string,
      loadedFileName = "family-tree.json",
      path: string | null = null,
    ) => {
      if (!isWasmLoaded) return;
      setError(null);
      resetSaveStatus();
      hasTreeRef.current = false;

      Effect.runPromise(
        Effect.gen(function* () {
          setTreeData(null);
          yield* WasmServiceLive.loadTree(fileContent);
          const data = yield* WasmServiceLive.getTreeData();
          const persons = yield* WasmServiceLive.getFullPersons();
          setTreeData(data);
          setFullTreePersons(persons);
          setTreeViewSelection(null);
          setTreeKey((k) => k + 1);
          fileNameRef.current = withJsonExtension(loadedFileName);
          currentPathRef.current = path;
          hasTreeRef.current = true;
        }),
      ).catch((err) => {
        setError(`Failed to load tree: ${err.message}`);
      });
    },
    [isWasmLoaded, resetSaveStatus],
  );

  const handleSaveTree = useCallback(
    async (saveAs = false) => {
      if (!isWasmLoaded || !hasTreeRef.current) {
        setError("Load a tree before saving.");
        return;
      }
      if (savingRef.current) return;

      setError(null);
      resetSaveStatus();
      savingRef.current = true;
      setSaveStatus("saving");
      try {
        const content = await Effect.runPromise(WasmServiceLive.saveTree());

        if (!isTauri()) {
          downloadTree(content, fileNameRef.current);
          showSavedStatus();
          return;
        }

        let path = saveAs ? null : currentPathRef.current;
        if (path === null) {
          path = await showSaveDialog({
            title: "Save Family Tree",
            defaultPath: currentPathRef.current ?? fileNameRef.current,
            filters: [{ name: "Family tree JSON", extensions: ["json"] }],
          });
        }
        if (path === null) {
          resetSaveStatus();
          return;
        }

        path = withJsonExtension(path);
        await invoke("save_as", { path, content });
        currentPathRef.current = path;
        fileNameRef.current = fileNameFromPath(path);
        showSavedStatus();
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setError(`Failed to save tree: ${message}`);
        resetSaveStatus();
      } finally {
        savingRef.current = false;
      }
    },
    [isWasmLoaded, resetSaveStatus, showSavedStatus],
  );

  const handleCreateTree = useCallback(() => {
    if (!isWasmLoaded) return;

    setError(null);
    resetSaveStatus();
    hasTreeRef.current = false;

    Effect.runPromise(
      Effect.gen(function* () {
        yield* WasmServiceLive.newTree;
        const data = yield* WasmServiceLive.getTreeData();
        const persons = yield* WasmServiceLive.getFullPersons();
        setTreeData(data);
        setFullTreePersons(persons);
        setTreeViewSelection(null);
        setTreeKey((k) => k + 1);
        fileNameRef.current = "family-tree.json";
        currentPathRef.current = null;
        hasTreeRef.current = true;
      }),
    ).catch((err) => {
      setError(`Failed to create tree: ${err.message}`);
    });
  }, [isWasmLoaded, resetSaveStatus]);

  useEffect(() => {
    if (isTauri()) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || event.altKey) return;

      if (event.key.toLowerCase() === "o" && !event.shiftKey) {
        event.preventDefault();
        loadTreeDialogRef.current?.openPicker();
      } else if (event.key.toLowerCase() === "s") {
        event.preventDefault();
        void handleSaveTree(event.shiftKey);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleSaveTree]);

  useEffect(() => {
    if (!isTauri() || !isWasmLoaded) return;

    let disposed = false;
    const unlistenFunctions: UnlistenFn[] = [];
    const addListener = async <T,>(
      eventName: string,
      handler: (payload: T) => void,
    ) => {
      const unlisten = await listen<T>(eventName, (event) =>
        handler(event.payload),
      );
      if (disposed) {
        unlisten();
      } else {
        unlistenFunctions.push(unlisten);
      }
    };

    void Promise.all([
      addListener<OpenTreePayload>("open", ({ path, content }) => {
        handleLoadTree(content, fileNameFromPath(path), path);
      }),
      addListener<string>("open-error", (message) => {
        setError(`Failed to open tree: ${message}`);
      }),
      addListener<void>("save", () => {
        void handleSaveTree(false);
      }),
      addListener<void>("save-as", () => {
        void handleSaveTree(true);
      }),
    ]).catch((err) => {
      const message = err instanceof Error ? err.message : String(err);
      setError(`Failed to register desktop menu actions: ${message}`);
    });

    return () => {
      disposed = true;
      unlistenFunctions.forEach((unlisten) => unlisten());
    };
  }, [handleLoadTree, handleSaveTree, isWasmLoaded]);

  const handleRefresh = () => {
    if (!isWasmLoaded) return;
    Effect.runPromise(
      Effect.gen(function* () {
        const data = yield* WasmServiceLive.getTreeData();
        const persons = yield* WasmServiceLive.getFullPersons();
        setTreeData(data);
        setFullTreePersons(persons);
      }),
    ).catch((err) => {
      console.error(err);
      setError(`Failed to refresh tree: ${err.message}`);
    });
  };

  const handleSetPartialView = useCallback(
    (root: string, scope: TreeViewScope) => {
      if (!isWasmLoaded) return;
      setError(null);

      Effect.runPromise(
        Effect.gen(function* () {
          yield* WasmServiceLive.setPartialView(
            root,
            viewOptionsForScope(scope),
          );
          const data = yield* WasmServiceLive.getTreeData();
          setTreeData(data);
          setTreeViewSelection({ root, scope });
        }),
      ).catch((err) => {
        setError(`Failed to filter tree: ${err.message}`);
      });
    },
    [isWasmLoaded],
  );

  const handleSetFullView = useCallback(() => {
    if (!isWasmLoaded) return;
    setError(null);

    Effect.runPromise(
      Effect.gen(function* () {
        yield* WasmServiceLive.setFullView;
        const data = yield* WasmServiceLive.getTreeData();
        setTreeData(data);
        setTreeViewSelection(null);
      }),
    ).catch((err) => {
      setError(`Failed to restore full tree: ${err.message}`);
    });
  }, [isWasmLoaded]);

  if (!isWasmLoaded && !error) {
    return (
      <div className="w-screen h-screen flex items-center justify-center bg-background">
        <div className="flex flex-col items-center gap-4">
          <div className="animate-spin w-8 h-8 border-4 border-primary border-t-transparent rounded-full" />
          <p className="text-muted-foreground animate-pulse">
            Initializing WASM Module...
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-dvh w-screen flex-col overflow-hidden bg-background text-foreground">
      {/* Header */}
      <header className="relative z-10 flex shrink-0 flex-none flex-col gap-2 border-b border-border bg-card px-3 py-2 shadow-sm sm:h-16 sm:flex-row sm:items-center sm:justify-between sm:gap-4 sm:px-6 sm:py-0">
        <div className="flex min-w-0 items-center justify-between gap-2 sm:shrink-0">
          <div className="flex min-w-0 items-center gap-2 sm:gap-3">
            <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-primary/10">
              <svg
                className="h-5 w-5 text-primary"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
              </svg>
            </div>
            <div className="min-w-0">
              <h1 className="truncate font-sans text-lg font-bold tracking-tight text-foreground/90 sm:text-xl">
                Baumstamm
              </h1>
              <p className="hidden text-xs font-medium uppercase tracking-widest text-muted-foreground sm:block">
                Family Tree Viewer
              </p>
            </div>
          </div>
          {error && (
            <span
              className="max-w-[55vw] truncate rounded-full bg-destructive/10 px-2 py-1 text-xs font-medium text-destructive sm:hidden"
              title={error}
            >
              {error}
            </span>
          )}
        </div>
        <div className="grid min-w-0 grid-cols-[minmax(0,1fr)_2.75rem_2.75rem] items-center gap-2 sm:flex sm:gap-4">
          {error && (
            <span className="hidden max-w-xs truncate rounded-full bg-destructive/10 px-3 py-1 text-sm font-medium text-destructive sm:block">
              {error}
            </span>
          )}
          <div
            className="grid grid-cols-2 rounded-md border border-border bg-background p-1 sm:flex"
            role="group"
            aria-label="Choose main view"
          >
            <Button
              type="button"
              size="sm"
              variant={mainView === "tree" ? "secondary" : "ghost"}
              aria-pressed={mainView === "tree"}
              onClick={() => setMainView("tree")}
            >
              <Network className="h-4 w-4" />
              Tree
            </Button>
            <Button
              type="button"
              size="sm"
              variant={mainView === "table" ? "secondary" : "ghost"}
              aria-pressed={mainView === "table"}
              onClick={() => setMainView("table")}
            >
              <TableProperties className="h-4 w-4" />
              Table
            </Button>
          </div>
          <Button
            onClick={() => void handleSaveTree(false)}
            variant="outline"
            disabled={treeData === null || saveStatus === "saving"}
            aria-live="polite"
            className={
              saveStatus === "saved"
                ? "h-11 w-11 border-emerald-500 bg-emerald-500/10 px-0 text-emerald-700 hover:bg-emerald-500/10 hover:text-emerald-700 dark:text-emerald-400 sm:h-9 sm:w-28 sm:px-4"
                : "h-11 w-11 px-0 sm:h-9 sm:w-28 sm:px-4"
            }
            title={
              saveStatus === "saving"
                ? "Saving tree"
                : saveStatus === "saved"
                  ? "Tree saved"
                  : "Save tree (Ctrl/Command+S; add Shift for Save As)"
            }
            aria-keyshortcuts="Control+S Meta+S Control+Shift+S Meta+Shift+S"
          >
            {saveStatus === "saving" ? (
              <LoaderCircle className="h-4 w-4 animate-spin sm:mr-2" />
            ) : saveStatus === "saved" ? (
              <Check className="h-4 w-4 sm:mr-2" />
            ) : isTauri() ? (
              <Save className="h-4 w-4 sm:mr-2" />
            ) : (
              <Download className="h-4 w-4 sm:mr-2" />
            )}
            <span className="sr-only sm:not-sr-only">
              {saveStatus === "saving"
                ? "Saving…"
                : saveStatus === "saved"
                  ? "Saved"
                  : "Save Tree"}
            </span>
          </Button>
          <LoadTreeDialog ref={loadTreeDialogRef} onLoad={handleLoadTree} />
        </div>
      </header>

      {/* Main Content Area */}
      <main className="flex-1 min-h-0 relative">
        {mainView === "tree" ? (
          <TreeCanvas
            key={treeKey}
            data={treeData}
            viewSelection={treeViewSelection}
            onCreate={handleCreateTree}
            onUpdate={handleRefresh}
            onSetPartialView={handleSetPartialView}
            onSetFullView={handleSetFullView}
          />
        ) : (
          <PersonTable
            key={treeKey}
            data={treeData}
            fullTreePersons={fullTreePersons}
            viewSelection={treeViewSelection}
            onCreate={handleCreateTree}
            onUpdate={handleRefresh}
            onSetPartialView={handleSetPartialView}
            onSetFullView={handleSetFullView}
          />
        )}
      </main>
    </div>
  );
}

export default App;
