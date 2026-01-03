import React from 'react';
import { WidgetNode } from '../types';
import { EventHandler, renderWidget } from './index';

interface ColumnWidgetProps {
    node: WidgetNode;
    onEvent: EventHandler;
}

/**
 * Column widget - arranges children vertically using flexbox
 */
function ColumnWidget({ node, onEvent }: ColumnWidgetProps) {
    return (
        <div style={styles.column}>
            {node.children.map((child) => renderWidget(child, onEvent))}
        </div>
    );
}

const styles: Record<string, React.CSSProperties> = {
    column: {
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'stretch',
        gap: '10px',
        padding: '5px 0',
    },
};

export default ColumnWidget;
