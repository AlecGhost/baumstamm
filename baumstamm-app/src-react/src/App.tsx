import { useEffect, useState } from "react";
import { Effect } from "effect";
import { WasmServiceLive } from "@/lib/wasm";
import type { TreeData } from "@/lib/types";
import { LoadTreeDialog } from "@/components/LoadTreeDialog";
import { TreeCanvas } from "@/components/TreeCanvas";

function App() {
  const [isWasmLoaded, setIsWasmLoaded] = useState<boolean>(false);
  const [treeData, setTreeData] = useState<TreeData | null>(null);
  const [error, setError] = useState<string | null>(null);

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

  const handleLoadTree = (fileContent: string) => {
    if (!isWasmLoaded) return;
    setError(null);

    Effect.runPromise(
      Effect.gen(function* () {
        setTreeData(null);
        yield* WasmServiceLive.loadTree(fileContent);
        const data = yield* WasmServiceLive.getTreeData();
        setTreeData(data);
      }),
    ).catch((err) => {
      setError(`Failed to load tree: ${err.message}`);
    });
  };

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
          <LoadTreeDialog onLoad={handleLoadTree} />
        </div>
      </header>

      {/* Main Content Area */}
      <main className="flex-1 min-h-0 relative">
        <TreeCanvas data={treeData} onUpdate={handleRefresh} />
      </main>
    </div>
  );
}

export default App;
