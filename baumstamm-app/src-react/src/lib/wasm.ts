import { Effect, Context } from "effect";
import init, {
  load_tree,
  get_persons,
  get_relationships,
  get_grid,
  get_tree_data,
  get_sub_tree_data,
  insert_info,
  remove_info,
} from "baumstamm-wasm";
import type { Person, Relationship, Grid, TreeData, ViewOptions } from "./types";

export interface WasmService {
  readonly init: Effect.Effect<void, Error>;
  readonly loadTree: (input: string) => Effect.Effect<void, Error>;
  readonly getPersons: () => Effect.Effect<Person[], Error>;
  readonly getRelationships: () => Effect.Effect<Relationship[], Error>;
  readonly getGrid: () => Effect.Effect<Grid, Error>;
  readonly getTreeData: () => Effect.Effect<TreeData, Error>;
  readonly getSubTreeData: (
    root: string,
    options: ViewOptions,
  ) => Effect.Effect<TreeData, Error>;
  readonly insertInfo: (
    pid: string,
    key: string,
    value: string,
  ) => Effect.Effect<void, Error>;
  readonly removeInfo: (pid: string, key: string) => Effect.Effect<void, Error>;
}

export const WasmService = Context.GenericTag<WasmService>(
  "@services/WasmService",
);

export const WasmServiceLive = {
  init: Effect.tryPromise({
    try: () => init(),
    catch: (error) => new Error(`Failed to initialize WASM: ${error}`),
  }).pipe(Effect.asVoid),

  loadTree: (input: string) =>
    Effect.try({
      try: () => {
        load_tree(input);
      },
      catch: (error) => new Error(`Failed to load tree: ${error}`),
    }),

  getPersons: () =>
    Effect.try({
      try: () => get_persons() as Person[],
      catch: (error) => new Error(`Failed to get persons: ${error}`),
    }),

  getRelationships: () =>
    Effect.try({
      try: () => get_relationships() as Relationship[],
      catch: (error) => new Error(`Failed to get relationships: ${error}`),
    }),

  getGrid: () =>
    Effect.try({
      try: () => get_grid() as Grid,
      catch: (error) => new Error(`Failed to get grid: ${error}`),
    }),

  insertInfo: (pid: string, key: string, value: string) =>
    Effect.try({
      try: () => {
        insert_info(pid, key, value);
      },
      catch: (error) => new Error(`Failed to insert info: ${error}`),
    }),

  removeInfo: (pid: string, key: string) =>
    Effect.try({
      try: () => {
        remove_info(pid, key);
      },
      catch: (error) => new Error(`Failed to remove info: ${error}`),
    }),

  getTreeData: () =>
    Effect.try({
      try: () => get_tree_data() as TreeData,
      catch: (error) => new Error(`Failed to get tree data: ${error}`),
    }),

  getSubTreeData: (root: string, options: ViewOptions) =>
    Effect.try({
      try: () => get_sub_tree_data(root, options) as TreeData,
      catch: (error) => new Error(`Failed to get sub tree data: ${error}`),
    }),
};
