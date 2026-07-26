import { Effect, Context } from "effect";
import init, {
  load_tree,
  new_tree,
  save_tree,
  get_persons,
  get_relationships,
  get_grid,
  get_tree_data,
  get_sub_tree_data,
  set_partial_view,
  set_full_view,
  insert_info,
  remove_info,
  add_parent,
  add_child,
  add_new_relationship,
  add_relationship_with_partner,
  remove_person,
  merge_person,
} from "baumstamm-wasm";
import type {
  Person,
  Relationship,
  Grid,
  TreeData,
  ViewOptions,
} from "./types";

export interface WasmService {
  readonly init: Effect.Effect<void, Error>;
  readonly loadTree: (input: string) => Effect.Effect<void, Error>;
  readonly newTree: Effect.Effect<void, Error>;
  readonly saveTree: () => Effect.Effect<string, Error>;
  readonly getPersons: () => Effect.Effect<Person[], Error>;
  readonly getRelationships: () => Effect.Effect<Relationship[], Error>;
  readonly getGrid: () => Effect.Effect<Grid, Error>;
  readonly getTreeData: () => Effect.Effect<TreeData, Error>;
  readonly getSubTreeData: (
    root: string,
    options: ViewOptions,
  ) => Effect.Effect<TreeData, Error>;
  readonly setPartialView: (
    root: string,
    options: ViewOptions,
  ) => Effect.Effect<void, Error>;
  readonly setFullView: Effect.Effect<void, Error>;
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

export const WasmServiceLive: WasmService = {
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

  newTree: Effect.try({
    try: () => {
      new_tree();
    },
    catch: (error) => new Error(`Failed to create tree: ${error}`),
  }),

  saveTree: () =>
    Effect.try({
      try: () => save_tree() as string,
      catch: (error) => new Error(`Failed to save tree: ${error}`),
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

  setPartialView: (root: string, options: ViewOptions) =>
    Effect.try({
      try: () => {
        set_partial_view(root, options);
      },
      catch: (error) => new Error(`Failed to set partial view: ${error}`),
    }),

  setFullView: Effect.try({
    try: () => {
      set_full_view();
    },
    catch: (error) => new Error(`Failed to set full view: ${error}`),
  }),

  addParent: (rid: string) =>
    Effect.try({
      try: () => add_parent(rid) as [string, string],
      catch: (error) => new Error(`Failed to add parent: ${error}`),
    }),

  addChild: (rid: string) =>
    Effect.try({
      try: () => add_child(rid) as string,
      catch: (error) => new Error(`Failed to add child: ${error}`),
    }),

  addNewRelationship: (pid: string) =>
    Effect.try({
      try: () => add_new_relationship(pid) as string,
      catch: (error) => new Error(`Failed to add relationship: ${error}`),
    }),

  addRelationshipWithPartner: (pid: string, partnerPid: string) =>
    Effect.try({
      try: () => add_relationship_with_partner(pid, partnerPid) as string,
      catch: (error) =>
        new Error(`Failed to add relationship with partner: ${error}`),
    }),

  removePerson: (pid: string) =>
    Effect.try({
      try: () => {
        remove_person(pid);
      },
      catch: (error) => new Error(`Failed to remove person: ${error}`),
    }),

  mergePerson: (pid1: string, pid2: string) =>
    Effect.try({
      try: () => {
        merge_person(pid1, pid2);
      },
      catch: (error) => new Error(`Failed to merge person: ${error}`),
    }),
};
