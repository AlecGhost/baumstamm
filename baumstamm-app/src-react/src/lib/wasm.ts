import { Context, Effect } from "effect";
import { createDelayedActivitySource, type ActivitySource } from "./activity";
import type {
  Grid,
  Person,
  Relationship,
  TreeData,
  ViewOptions,
} from "./types";
import { WasmRpcClient } from "./wasm-rpc-client";
import type { TreeSnapshot, WasmRpcMethod, WasmRpcMethods } from "./wasm-rpc";

export interface WasmService {
  readonly init: Effect.Effect<void, Error>;
  readonly loadTree: (input: string) => Effect.Effect<void, Error>;
  readonly loadTreeSnapshot: (
    input: string,
  ) => Effect.Effect<TreeSnapshot, Error>;
  readonly newTree: Effect.Effect<void, Error>;
  readonly newTreeSnapshot: Effect.Effect<TreeSnapshot, Error>;
  readonly saveTree: () => Effect.Effect<string, Error>;
  readonly getPersons: () => Effect.Effect<Person[], Error>;
  readonly getFullPersons: () => Effect.Effect<Person[], Error>;
  readonly getRelationships: () => Effect.Effect<Relationship[], Error>;
  readonly getGrid: () => Effect.Effect<Grid, Error>;
  readonly getTreeData: () => Effect.Effect<TreeData, Error>;
  readonly getTreeSnapshot: () => Effect.Effect<TreeSnapshot, Error>;
  readonly getSubTreeData: (
    root: string,
    options: ViewOptions,
  ) => Effect.Effect<TreeData, Error>;
  readonly setPartialView: (
    root: string,
    options: ViewOptions,
  ) => Effect.Effect<void, Error>;
  readonly setPartialViewSnapshot: (
    root: string,
    options: ViewOptions,
  ) => Effect.Effect<TreeData, Error>;
  readonly setFullView: Effect.Effect<void, Error>;
  readonly setFullViewSnapshot: Effect.Effect<TreeData, Error>;
  readonly insertInfo: (
    pid: string,
    key: string,
    value: string,
  ) => Effect.Effect<void, Error>;
  readonly removeInfo: (pid: string, key: string) => Effect.Effect<void, Error>;
  readonly addParent: (rid: string) => Effect.Effect<[string, string], Error>;
  readonly addChild: (rid: string) => Effect.Effect<string, Error>;
  readonly addNewRelationship: (pid: string) => Effect.Effect<string, Error>;
  readonly addRelationshipWithPartner: (
    pid: string,
    partnerPid: string,
  ) => Effect.Effect<string, Error>;
  readonly removePerson: (pid: string) => Effect.Effect<void, Error>;
  readonly mergePerson: (
    pid1: string,
    pid2: string,
  ) => Effect.Effect<void, Error>;
}

export const WasmService = Context.GenericTag<WasmService>(
  "@services/WasmService",
);

const rpcClient = new WasmRpcClient(
  () =>
    new Worker(new URL("../workers/wasm.worker.ts", import.meta.url), {
      type: "module",
      name: "baumstamm-wasm",
    }),
);

const delayedActivity = createDelayedActivitySource(rpcClient.activity, 200);
export const wasmActivity: ActivitySource = delayedActivity;

const rpcEffect = <Method extends WasmRpcMethod>(
  label: string,
  method: Method,
  ...args: WasmRpcMethods[Method]["args"]
): Effect.Effect<WasmRpcMethods[Method]["result"], Error> =>
  Effect.tryPromise({
    try: () => rpcClient.call(method, ...args),
    catch: (error) =>
      new Error(
        `Failed to ${label}: ${
          error instanceof Error ? error.message : String(error)
        }`,
      ),
  });

export const WasmServiceLive: WasmService = {
  init: rpcEffect("initialize WASM", "init"),
  loadTree: (input) => rpcEffect("load tree", "loadTree", input),
  loadTreeSnapshot: (input) =>
    rpcEffect("load tree", "loadTreeSnapshot", input),
  newTree: rpcEffect("create tree", "newTree"),
  newTreeSnapshot: rpcEffect("create tree", "newTreeSnapshot"),
  saveTree: () => rpcEffect("save tree", "saveTree"),
  getPersons: () => rpcEffect("get persons", "getPersons"),
  getFullPersons: () => rpcEffect("get full person list", "getFullPersons"),
  getRelationships: () => rpcEffect("get relationships", "getRelationships"),
  getGrid: () => rpcEffect("get grid", "getGrid"),
  getTreeData: () => rpcEffect("get tree data", "getTreeData"),
  getTreeSnapshot: () => rpcEffect("get tree data", "getTreeSnapshot"),
  getSubTreeData: (root, options) =>
    rpcEffect("get sub tree data", "getSubTreeData", root, options),
  setPartialView: (root, options) =>
    rpcEffect("set partial view", "setPartialView", root, options),
  setPartialViewSnapshot: (root, options) =>
    rpcEffect("set partial view", "setPartialViewSnapshot", root, options),
  setFullView: rpcEffect("set full view", "setFullView"),
  setFullViewSnapshot: rpcEffect("set full view", "setFullViewSnapshot"),
  insertInfo: (pid, key, value) =>
    rpcEffect("insert info", "insertInfo", pid, key, value),
  removeInfo: (pid, key) => rpcEffect("remove info", "removeInfo", pid, key),
  addParent: (rid) => rpcEffect("add parent", "addParent", rid),
  addChild: (rid) => rpcEffect("add child", "addChild", rid),
  addNewRelationship: (pid) =>
    rpcEffect("add relationship", "addNewRelationship", pid),
  addRelationshipWithPartner: (pid, partnerPid) =>
    rpcEffect(
      "add relationship with partner",
      "addRelationshipWithPartner",
      pid,
      partnerPid,
    ),
  removePerson: (pid) => rpcEffect("remove person", "removePerson", pid),
  mergePerson: (pid1, pid2) =>
    rpcEffect("merge person", "mergePerson", pid1, pid2),
};

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    delayedActivity.dispose();
    rpcClient.dispose();
  });
}
