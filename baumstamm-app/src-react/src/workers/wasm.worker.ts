import init, {
  add_child,
  add_new_relationship,
  add_parent,
  add_relationship_with_partner,
  get_full_persons,
  get_grid,
  get_persons,
  get_relationships,
  get_sub_tree_data,
  get_tree_data,
  insert_info,
  load_tree,
  merge_person,
  new_tree,
  remove_info,
  remove_person,
  save_tree,
  set_full_view,
  set_partial_view,
} from "baumstamm-wasm";
import { SerialTaskQueue } from "../lib/serial-task-queue";
import type {
  SerializedWorkerError,
  TreeSnapshot,
  WasmRpcRequest,
  WasmRpcResponse,
} from "../lib/wasm-rpc";
import type { Grid, Person, Relationship, TreeData } from "../lib/types";

type WorkerScope = {
  addEventListener(
    type: "message",
    listener: (event: MessageEvent<WasmRpcRequest>) => void,
  ): void;
  postMessage(message: WasmRpcResponse): void;
};

const workerScope = globalThis as unknown as WorkerScope;
const queue = new SerialTaskQueue();
let initialization: Promise<void> | null = null;

const ensureInitialized = () => {
  initialization ??= init().then(() => undefined);
  return initialization;
};

const getTreeSnapshot = (): TreeSnapshot => ({
  treeData: get_tree_data() as TreeData,
  fullPersons: get_full_persons() as Person[],
});

const execute = async (request: WasmRpcRequest): Promise<unknown> => {
  await ensureInitialized();

  switch (request.method) {
    case "init":
      return undefined;
    case "loadTree":
      return load_tree(...request.args);
    case "loadTreeSnapshot":
      load_tree(...request.args);
      return getTreeSnapshot();
    case "newTree":
      return new_tree();
    case "newTreeSnapshot":
      new_tree();
      return getTreeSnapshot();
    case "saveTree":
      return save_tree() as string;
    case "getPersons":
      return get_persons() as Person[];
    case "getFullPersons":
      return get_full_persons() as Person[];
    case "getRelationships":
      return get_relationships() as Relationship[];
    case "getGrid":
      return get_grid() as Grid;
    case "getTreeData":
      return get_tree_data() as TreeData;
    case "getTreeSnapshot":
      return getTreeSnapshot();
    case "getSubTreeData":
      return get_sub_tree_data(...request.args) as TreeData;
    case "setPartialView":
      return set_partial_view(...request.args);
    case "setPartialViewSnapshot":
      set_partial_view(...request.args);
      return get_tree_data() as TreeData;
    case "setFullView":
      return set_full_view();
    case "setFullViewSnapshot":
      set_full_view();
      return get_tree_data() as TreeData;
    case "insertInfo":
      return insert_info(...request.args);
    case "removeInfo":
      return remove_info(...request.args);
    case "addParent":
      return add_parent(...request.args) as [string, string];
    case "addChild":
      return add_child(...request.args) as string;
    case "addNewRelationship":
      return add_new_relationship(...request.args) as string;
    case "addRelationshipWithPartner":
      return add_relationship_with_partner(...request.args) as string;
    case "removePerson":
      return remove_person(...request.args);
    case "mergePerson":
      return merge_person(...request.args);
  }
};

const serializeError = (error: unknown): SerializedWorkerError => {
  if (error instanceof Error) {
    return { name: error.name, message: error.message, stack: error.stack };
  }
  return { name: "Error", message: String(error) };
};

workerScope.addEventListener("message", (event) => {
  const { id } = event.data;
  void queue
    .enqueue(() => execute(event.data))
    .then(
      (value) => workerScope.postMessage({ id, ok: true, value }),
      (error) =>
        workerScope.postMessage({
          id,
          ok: false,
          error: serializeError(error),
        }),
    );
});
