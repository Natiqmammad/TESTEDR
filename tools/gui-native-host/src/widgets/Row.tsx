import React from 'react';
import { WidgetNode } from '../types';
import { EventHandler, renderWidget } from './index';

interface RowWidgetProps {
    node: WidgetNode;
    onEvent: EventHandler;
}

/**
 * Row widget - arranges children horizontally using flexbox
 */
function RowWidget({ node, onEvent }: RowWidgetProps) {
    return (
        <div style={styles.row}>
            {node.children.map((child) => renderWidget(child, onEvent))}
        </div>
    );
}

const styles: Record<string, React.CSSProperties> = {
    row: {
        display: 'flex',
        flexDirection: 'row',
        alignItems: 'center',
        gap: '10px',
        padding: '5px 0',
    },
};

export default RowWidget;
