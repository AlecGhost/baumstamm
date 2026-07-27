import type {
  Grid,
  GridLayoutAlgorithm,
  Person,
  Relationship,
  TreeData,
  ViewOptions,
} from "./types";

export type TreeSnapshot = {
  treeData: TreeData;
  fullPersons: Person[];
};

export interface WasmRpcMethods {
  init: { args: []; result: void };
  loadTree: { args: [input: string]; result: void };
  loadTreeSnapshot: { args: [input: string]; result: TreeSnapshot };
  newTree: { args: []; result: void };
  newTreeSnapshot: { args: []; result: TreeSnapshot };
  saveTree: { args: []; result: string };
  saveSubTree: {
    args: [root: string, options: ViewOptions];
    result: string;
  };
  getPersons: { args: []; result: Person[] };
  getFullPersons: { args: []; result: Person[] };
  getRelationships: { args: []; result: Relationship[] };
  getGrid: { args: []; result: Grid };
  getTreeData: { args: []; result: TreeData };
  getTreeSnapshot: { args: []; result: TreeSnapshot };
  getSubTreeData: {
    args: [root: string, options: ViewOptions];
    result: TreeData;
  };
  setPartialView: {
    args: [root: string, options: ViewOptions];
    result: void;
  };
  setPartialViewSnapshot: {
    args: [root: string, options: ViewOptions];
    result: TreeData;
  };
  setFullView: { args: []; result: void };
  setFullViewSnapshot: { args: []; result: TreeData };
  setGridLayoutSnapshot: {
    args: [layoutAlgorithm: GridLayoutAlgorithm];
    result: TreeData;
  };
  insertInfo: {
    args: [pid: string, key: string, value: string];
    result: void;
  };
  removeInfo: { args: [pid: string, key: string]; result: void };
  addParent: { args: [rid: string]; result: [string, string] };
  addChild: { args: [rid: string]; result: string };
  addNewRelationship: { args: [pid: string]; result: string };
  addRelationshipWithPartner: {
    args: [pid: string, partnerPid: string];
    result: string;
  };
  removePerson: { args: [pid: string]; result: void };
  mergePerson: { args: [pid1: string, pid2: string]; result: void };
}

export type WasmRpcMethod = keyof WasmRpcMethods;

export type WasmRpcRequest = {
  [Method in WasmRpcMethod]: {
    id: number;
    method: Method;
    args: WasmRpcMethods[Method]["args"];
  };
}[WasmRpcMethod];

export type SerializedWorkerError = {
  name: string;
  message: string;
  stack?: string;
};

export type WasmRpcResponse =
  | { id: number; ok: true; value: unknown }
  | { id: number; ok: false; error: SerializedWorkerError };
