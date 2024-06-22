import init, {
    get_persons,
    get_relationships,
    get_grid,
    init_state,
    load_tree,
} from "./target/baumstamm-wasm/baumstamm_wasm.js";

const flags = {
    isTauri: "__TAURI__" in window,
};
const app = Elm.Main.init({
    node: document.getElementById("baumstamm"),
    flags,
});
await init();
var state = init_state();
const incomingProcs = {
    new: "new",
    load: "load",
    getTreeData: "get_tree_data",
};
const outgoingProcs = {
    treeData: "tree_data",
    error: "error",
};
app.ports.send.subscribe((rpc) => {
    try {
        switch (rpc.proc) {
            case incomingProcs.new:
                {
                    state = init_state();
                    let treeData = getTreeData(state);
                    send(outgoingProcs.treeData, treeData);
                }
                break;
            case incomingProcs.load:
                {
                    load_tree(rpc.payload, state);
                    let treeData = getTreeData(state);
                    send(outgoingProcs.treeData, treeData);
                }
                break;
            case incomingProcs.getTreeData:
                {
                    let treeData = getTreeData(state);
                    send(outgoingProcs.treeData, treeData);
                }
                break;
            default:
                send(outgoingProcs.error, "The unknown procedure '" + rpc.proc + "' was called.");
        }
    } catch (e) {
        send(outgoingProcs.error, e);
    }
});

function send(proc, payload) {
    app.ports.receive.send({
        proc,
        payload,
    });
}

function getTreeData(state) {
    const persons = get_persons(state).map(person => {
        // convert Map to Object
        const id = person.id;
        let info = null;
        if (person.info !== null && person.info !== undefined) {
            info = Object.fromEntries(person.info);
        }
        return { id, info };
    });
    const relationships = get_relationships(state);
    const grid = get_grid(state);
    return { persons, relationships, grid }
}
