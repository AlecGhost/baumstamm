import React, { useEffect, useState } from 'react';
import init, { State, init_state, get_grid } from 'baumstamm-wasm';

function App() {
    const [state, setState] = useState<State | null>(null);
    useEffect(() => { init().then((_) => setState(init_state())) }, []);
    return (
        <React.Suspense fallback={<div>Initialising App</div>}>
            {state && <div>Hello</div>}
        </React.Suspense >
    );
}

export default App;
