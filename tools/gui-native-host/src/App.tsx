import { useState, useEffect, useCallback } from 'react';
import { WidgetNode, RenderMessage, parseMessage, createEventMessage } from './types';
import { renderWidget } from './widgets';

// Demo mode: simulates runtime communication when running standalone
const DEMO_MODE = true;

function App() {
    const [tree, setTree] = useState<WidgetNode | null>(null);
    const [counter, setCounter] = useState(0);

    // In demo mode, create a sample widget tree
    useEffect(() => {
        if (DEMO_MODE) {
            const demoTree: WidgetNode = {
                id: 1,
                type: 'Column',
                props: {},
                children: [
                    {
                        id: 2,
                        type: 'Text',
                        props: { text: `count: ${counter}` },
                        children: [],
                    },
                    {
                        id: 3,
                        type: 'Button',
                        props: { label: 'Increment', handler: 'App.on_click' },
                        children: [],
                    },
                    {
                        id: 4,
                        type: 'Row',
                        props: {},
                        children: [
                            {
                                id: 5,
                                type: 'Button',
                                props: { label: '-5', handler: 'App.decrement' },
                                children: [],
                            },
                            {
                                id: 6,
                                type: 'Button',
                                props: { label: '+5', handler: 'App.increment_5' },
                                children: [],
                            },
                        ],
                    },
                    {
                        id: 7,
                        type: 'Container',
                        props: { padding: '20' },
                        children: [
                            {
                                id: 8,
                                type: 'Text',
                                props: { text: 'Hello from ApexForge GUI Native!' },
                                children: [],
                            },
                        ],
                    },
                ],
            };
            setTree(demoTree);
        }
    }, [counter]);

    // Handle widget events
    const handleEvent = useCallback((widgetId: number, eventType: string, handler: string) => {
        console.log(`Event: ${eventType} on widget ${widgetId}, handler: ${handler}`);

        if (DEMO_MODE) {
            // In demo mode, simulate handler execution
            if (handler === 'App.on_click' || handler === 'App.increment_5') {
                setCounter((c) => c + (handler === 'App.increment_5' ? 5 : 1));
            } else if (handler === 'App.decrement') {
                setCounter((c) => c - 5);
            }
        } else {
            // In production, send event to runtime via stdout
            const event = createEventMessage(eventType, widgetId, handler);
            console.log(JSON.stringify(event));
        }
    }, []);

    // In production mode, read from stdin
    useEffect(() => {
        if (!DEMO_MODE) {
            // This would be replaced with actual stdin reading in Node.js environment
            const handleLine = (line: string) => {
                const msg = parseMessage(line);
                if (msg?.kind === 'render') {
                    setTree((msg as RenderMessage).tree);
                }
            };

            // Placeholder: In Node.js, we'd use readline interface
            console.log('[gui-native-host] Ready to receive render messages');
        }
    }, []);

    return (
        <div style={styles.container}>
            <header style={styles.header}>
                <h1 style={styles.title}>ApexForge GUI Native Host</h1>
                <p style={styles.subtitle}>
                    {DEMO_MODE ? 'Demo Mode - Standalone Preview' : 'Connected to Runtime'}
                </p>
            </header>

            <main style={styles.main}>
                {tree ? (
                    <div style={styles.widgetContainer}>
                        {renderWidget(tree, handleEvent)}
                    </div>
                ) : (
                    <div style={styles.waiting}>
                        <p>Waiting for widget tree...</p>
                    </div>
                )}
            </main>

            <footer style={styles.footer}>
                <p>Counter value: {counter}</p>
            </footer>
        </div>
    );
}

const styles: Record<string, React.CSSProperties> = {
    container: {
        display: 'flex',
        flexDirection: 'column',
        minHeight: '100vh',
    },
    header: {
        padding: '20px',
        background: 'rgba(255, 255, 255, 0.05)',
        borderBottom: '1px solid rgba(255, 255, 255, 0.1)',
    },
    title: {
        fontSize: '24px',
        fontWeight: 600,
        color: '#fff',
        margin: 0,
    },
    subtitle: {
        fontSize: '14px',
        color: '#888',
        marginTop: '5px',
    },
    main: {
        flex: 1,
        padding: '30px',
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'flex-start',
    },
    widgetContainer: {
        background: 'rgba(255, 255, 255, 0.03)',
        borderRadius: '12px',
        padding: '30px',
        border: '1px solid rgba(255, 255, 255, 0.1)',
        minWidth: '400px',
    },
    waiting: {
        textAlign: 'center',
        color: '#888',
    },
    footer: {
        padding: '15px',
        textAlign: 'center',
        background: 'rgba(255, 255, 255, 0.02)',
        borderTop: '1px solid rgba(255, 255, 255, 0.1)',
        color: '#888',
        fontSize: '12px',
    },
};

export default App;
