import { Effect, Context } from "effect";
import init, {
    State,
    init_state,
    load_tree,
    get_persons,
    get_relationships,
    get_grid,
    // @ts-expect-error: baumstamm-wasm is untyped or loosely typed from wasm-pack
} from "baumstamm-wasm";
import type { Person, Relationship, Grid, TreeData } from "./types";

export interface WasmService {
    readonly init: Effect.Effect<void, Error>;
    readonly initState: Effect.Effect<State, Error>;
    readonly loadTree: (input: string, state: State) => Effect.Effect<void, Error>;
    readonly getPersons: (state: State) => Effect.Effect<Person[], Error>;
    readonly getRelationships: (state: State) => Effect.Effect<Relationship[], Error>;
    readonly getGrid: (state: State) => Effect.Effect<Grid, Error>;
    readonly getTreeData: (state: State) => Effect.Effect<TreeData, Error>;
}

export const WasmService = Context.GenericTag<WasmService>("@services/WasmService");

export const WasmServiceLive = {
    init: Effect.tryPromise({
        try: () => init(),
        catch: (error) => new Error(`Failed to initialize WASM: ${error}`),
    }).pipe(Effect.asVoid),

    initState: Effect.try({
        try: () => init_state(),
        catch: (error) => new Error(`Failed to initialize state: ${error}`),
    }),

    loadTree: (input: string, state: State) =>
        Effect.try({
            try: () => {
                load_tree(input, state);
            },
            catch: (error) => new Error(`Failed to load tree: ${error}`),
        }),

    getPersons: (state: State) =>
        Effect.try({
            try: () => {
                const rawPersons = get_persons(state);
                // Convert Map to Object for info
                return rawPersons.map((p: Record<string, any>) => ({
                    id: p.id,
                    info: p.info ? Object.fromEntries(p.info) : null,
                })) as Person[];
            },
            catch: (error) => new Error(`Failed to get persons: ${error}`),
        }),

    getRelationships: (state: State) =>
        Effect.try({
            try: () => get_relationships(state) as Relationship[],
            catch: (error) => new Error(`Failed to get relationships: ${error}`),
        }),

    getGrid: (state: State) =>
        Effect.try({
            try: () => get_grid(state) as Grid,
            catch: (error) => new Error(`Failed to get grid: ${error}`),
        }),

    getTreeData: function (state: State) {
        return Effect.all({
            persons: this.getPersons(state),
            relationships: this.getRelationships(state),
            grid: this.getGrid(state),
        }).pipe(
            Effect.map(({ persons, relationships, grid }) => ({
                persons,
                relationships,
                grid,
            }))
        );
    },
};
