import React, { useEffect, useState } from 'react';
import init, { State, init_state } from 'baumstamm-wasm';
import { SidebarProvider } from './components/ui/sidebar';
import { Sidebar } from './Sidebar';

function App() {
    const [state, setState] = useState<State | null>(null);
    const [open, setOpen] = React.useState(true);

    useEffect(() => { init().then((_) => setState(init_state())) }, []);

    const sideBarProps = {
        isOpen: open,
        toggle: () => setOpen((open) => !open),
    }

    return (
        <React.Suspense fallback={<div>Initialising App</div>}>
            <SidebarProvider open={open} onOpenChange={setOpen}>
                <Sidebar {...sideBarProps} />
                <main>
                    {state && <div>Hello</div>}
                </main>
            </SidebarProvider>
        </React.Suspense >
    );
}

export default App;
