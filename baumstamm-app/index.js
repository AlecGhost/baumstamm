import init, {
    get_persons,
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
    getPersons: "get_persons",
};
const outgoingProcs = {
    persons: "persons",
};
app.ports.send.subscribe((rpc) => {
    switch (rpc.proc) {
        case incomingProcs.new:
            {
                state = init_state();
                let persons = get_persons(state);
                send(outgoingProcs.persons, persons);
            }
        case incomingProcs.load:
            {
                load_tree(rpc.payload, state);
                let persons = get_persons(state);
                send(outgoingProcs.persons, persons);
            }
            break;
        case incomingProcs.getPersons:
            {
                let persons = get_persons(state);
                send(outgoingProcs.persons, persons);
            }
            break;
        default:
            console.log("Unknown method:", rpc.proc);
    }
});

function send(proc, payload) {
    app.ports.receive.send({
        proc,
        payload,
    });
}
