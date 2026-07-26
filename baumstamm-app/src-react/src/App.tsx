import { useCallback, useEffect, useRef, useState } from "react";
import { Effect } from "effect";
import { save as showSaveDialog } from "@tauri-apps/api/dialog";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/tauri";
import { Check, Download, LoaderCircle, Save } from "lucide-react";
import { WasmServiceLive } from "@/lib/wasm";
import type {
  TreeData,
  TreeViewScope,
  TreeViewSelection,
  ViewOptions,
} from "@/lib/types";
import { LoadTreeDialog } from "@/components/LoadTreeDialog";
import { TreeCanvas } from "@/components/TreeCanvas";
import { Button } from "@/components/ui/button";

type OpenTreePayload = {
  path: string;
  content: string;
};

type SaveStatus = "idle" | "saving" | "saved";

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
  const [error, setError] = useState<string | null>(null);
  const [treeKey, setTreeKey] = useState<number>(0);
  const [treeViewSelection, setTreeViewSelection] =
    useState<TreeViewSelection | null>(null);
  const [saveStatus, setSaveStatus] = useState<SaveStatus>("idle");
  const fileNameRef = useRef<string>("family-tree.json");
  const currentPathRef = useRef<string | null>(null);
  const hasTreeRef = useRef<boolean>(false);
  const savingRef = useRef<boolean>(false);
  const savedResetTimeoutRef = useRef<number | null>(null);

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
          setTreeData(data);
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
        setTreeData(data);
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
        setTreeData(data);
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
    <div className="w-screen h-screen flex flex-col overflow-hidden bg-background text-foreground">
      {/* Header */}
      <header className="h-16 flex-none border-b border-border bg-card px-6 flex items-center justify-between shrink-0 z-10 shadow-sm relative">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-primary/10 flex items-center justify-center">
            <svg
              className="w-5 h-5 text-primary"
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
          <div>
            <h1 className="text-xl font-bold font-sans tracking-tight text-foreground/90">
              Baumstamm
            </h1>
            <p className="text-xs text-muted-foreground font-medium uppercase tracking-widest">
              Family Tree Viewer
            </p>
          </div>
        </div>
        <div className="flex items-center gap-4">
          {error && (
            <span className="text-sm font-medium text-destructive px-3 py-1 bg-destructive/10 rounded-full">
              {error}
            </span>
          )}
          <Button
            onClick={() => void handleSaveTree(false)}
            variant="outline"
            disabled={treeData === null || saveStatus === "saving"}
            aria-live="polite"
            className={
              saveStatus === "saved"
                ? "w-28 border-emerald-500 bg-emerald-500/10 text-emerald-700 hover:bg-emerald-500/10 hover:text-emerald-700 dark:text-emerald-400"
                : "w-28"
            }
          >
            {saveStatus === "saving" ? (
              <LoaderCircle className="mr-2 h-4 w-4 animate-spin" />
            ) : saveStatus === "saved" ? (
              <Check className="mr-2 h-4 w-4" />
            ) : isTauri() ? (
              <Save className="mr-2 h-4 w-4" />
            ) : (
              <Download className="mr-2 h-4 w-4" />
            )}
            {saveStatus === "saving"
              ? "Saving…"
              : saveStatus === "saved"
                ? "Saved"
                : "Save Tree"}
          </Button>
          <LoadTreeDialog onLoad={handleLoadTree} />
        </div>
      </header>

      {/* Main Content Area */}
      <main className="flex-1 min-h-0 relative">
        <TreeCanvas
          key={treeKey}
          data={treeData}
          viewSelection={treeViewSelection}
          onCreate={handleCreateTree}
          onUpdate={handleRefresh}
          onSetPartialView={handleSetPartialView}
          onSetFullView={handleSetFullView}
        />
      </main>
    </div>
  );
}

export default App;
