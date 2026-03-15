import { Effect, Context } from "effect";
import init, {
  load_tree,
  get_persons,
  get_relationships,
  get_grid,
} from "baumstamm-wasm";
import type { Person, Relationship, Grid, TreeData } from "./types";

export interface WasmService {
  readonly init: Effect.Effect<void, Error>;
  readonly loadTree: (input: string) => Effect.Effect<void, Error>;
  readonly getPersons: () => Effect.Effect<Person[], Error>;
  readonly getRelationships: () => Effect.Effect<Relationship[], Error>;
  readonly getGrid: () => Effect.Effect<Grid, Error>;
  readonly getTreeData: () => Effect.Effect<TreeData, Error>;
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
      try: () => {
        const rawPersons = get_persons();
        // Convert Map to Object for info
        return rawPersons.map((p: Record<string, unknown>) => ({
          id: p.id,
          info: p.info ? Object.fromEntries(p.info) : null,
        })) as Person[];
      },
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

  getTreeData: function () {
    return Effect.all({
      persons: this.getPersons(),
      relationships: this.getRelationships(),
      grid: this.getGrid(),
    }).pipe(
      Effect.map(({ persons, relationships, grid }) => ({
        persons,
        relationships,
        grid,
      })),
    );
  },
};
