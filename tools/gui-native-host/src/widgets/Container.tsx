import React from 'react';
import { WidgetNode } from '../types';
import { EventHandler, renderWidget } from './index';

interface ContainerWidgetProps {
    node: WidgetNode;
    onEvent: EventHandler;
}

/**
 * Container widget - wraps children with padding and optional styling
 */
function ContainerWidget({ node, onEvent }: ContainerWidgetProps) {
    const padding = parseInt(node.props.padding || '0', 10);
    const background = node.props.background || 'rgba(255, 255, 255, 0.05)';

    const style: React.CSSProperties = {
        ...styles.container,
        padding: `${padding}px`,
        background,
    };

    return (
        <div style={style}>
            {node.children.map((child) => renderWidget(child, onEvent))}
        </div>
    );
}

const styles: Record<string, React.CSSProperties> = {
    container: {
        borderRadius: '8px',
        border: '1px solid rgba(255, 255, 255, 0.1)',
        margin: '5px 0',
    },
};

export default ContainerWidget;
