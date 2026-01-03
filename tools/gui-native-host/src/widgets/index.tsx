import React from 'react';
import { WidgetNode } from '../types';
import TextWidget from './Text';
import ButtonWidget from './Button';
import RowWidget from './Row';
import ColumnWidget from './Column';
import ContainerWidget from './Container';

export type EventHandler = (widgetId: number, eventType: string, handler: string) => void;

/**
 * Render a widget node based on its type
 */
export function renderWidget(
    node: WidgetNode,
    onEvent: EventHandler
): React.ReactNode {
    const key = `widget-${node.id}`;

    switch (node.type) {
        case 'Text':
            return <TextWidget key={key} node={node} />;

        case 'Button':
            return <ButtonWidget key={key} node={node} onEvent={onEvent} />;

        case 'Row':
            return <RowWidget key={key} node={node} onEvent={onEvent} />;

        case 'Column':
            return <ColumnWidget key={key} node={node} onEvent={onEvent} />;

        case 'Container':
            return <ContainerWidget key={key} node={node} onEvent={onEvent} />;

        default:
            return (
                <div key={key} style={{ color: '#ff6b6b', padding: '10px' }}>
                    Unknown widget type: {node.type}
                </div>
            );
    }
}

export { TextWidget, ButtonWidget, RowWidget, ColumnWidget, ContainerWidget };
