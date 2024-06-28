import init, {
    get_persons,
    get_relationships,
    get_grid,
    init_state,
    insert_info,
    load_tree,
    save_tree,
} from "./target/baumstamm-wasm/baumstamm_wasm.js";

// constants
const incomingProcs = {
    new: "new",
    load: "load",
    getTreeData: "get_tree_data",
    insertInfo: "insert_info",
};
const outgoingProcs = {
    treeData: "tree_data",
    error: "error",
};
const events = {
    open: "open",
    openError: "open-error",
    saveAs: "save-as",
}
const commands = {
    saveAs: "save_as",
}

// setup elm
const flags = {
    isTauri: "__TAURI__" in window,
};
const app = Elm.Main.init({
    node: document.getElementById("baumstamm"),
    flags,
});

// setup wasm
await init();
let state = init_state();

// tauri events
if (flags.isTauri) {
    const event = window.__TAURI__.event;
    const invoke = window.__TAURI__.invoke;
    event.listen(events.open, (event) => {
        const treeString = event.payload;
        try {
            load_tree(treeString, state);
            let treeData = getTreeData(state);
            send(outgoingProcs.treeData, treeData);
        } catch (e) {
            send(outgoingProcs.error, e);
        }
    });
    event.listen(events.openError, (event) => {
        const err = event.payload;
        send(outgoingProcs.error, err);
    });
    event.listen(events.saveAs, (event) => {
        const path = event.payload;
        const treeString = save_tree(state);
        invoke(commands.saveAs, { path, content: treeString })
            .catch((err) => send(outgoingProcs.error, err));
    });
}

// elm rpc
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
            case incomingProcs.insertInfo:
                {
                    let pid = rpc.payload.pid;
                    let key = rpc.payload.key;
                    let value = rpc.payload.value;
                    insert_info(pid, key, value, state);
                    let treeData = getTreeData(state);
                    send(outgoingProcs.treeData, treeData);
                }
                break;
            default:
                send(outgoingProcs.error, "The unknown procedure '" + rpc.proc + "' was called.");
        }
    } catch (e) {
        send(outgoingProcs.error, e.toString());
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
