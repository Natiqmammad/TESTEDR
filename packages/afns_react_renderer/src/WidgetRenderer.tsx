/**
 * AFNS Widget Tree Renderer
 * Converts VM widget values to React elements
 */

import React from 'react';
import { Text, Button, Row, Column, Container } from './components';

// Widget value type from VM
export interface WidgetValue {
    type: 'widget';
    id: string;
    widgetType: string;
    props: Map<string, unknown>;
    children: WidgetValue[];
    handlers: Map<string, unknown>;
}

export interface RenderContext {
    onEvent: (widgetId: string, eventType: string, payload?: unknown) => void;
}

/**
 * Get string prop from widget
 */
function getStringProp(widget: WidgetValue, key: string, defaultValue = ''): string {
    const val = widget.props.get(key);
    if (val === undefined || val === null) return defaultValue;
    if (typeof val === 'string') return val;
    if (typeof val === 'object' && 'type' in val) {
        const v = val as { type: string; value?: unknown };
        if (v.type === 'string' && typeof v.value === 'string') return v.value;
        if (v.type === 'int' && typeof v.value === 'bigint') return String(v.value);
        if (v.type === 'float' && typeof v.value === 'number') return String(v.value);
    }
    return String(val);
}

/**
 * Get number prop from widget
 */
function getNumberProp(widget: WidgetValue, key: string, defaultValue = 0): number {
    const val = widget.props.get(key);
    if (val === undefined || val === null) return defaultValue;
    if (typeof val === 'number') return val;
    if (typeof val === 'object' && 'type' in val) {
        const v = val as { type: string; value?: unknown };
        if (v.type === 'int' && typeof v.value === 'bigint') return Number(v.value);
        if (v.type === 'float' && typeof v.value === 'number') return v.value;
    }
    return defaultValue;
}

/**
 * Check if widget has a handler
 */
function hasHandler(widget: WidgetValue, eventType: string): boolean {
    return widget.handlers.has(eventType);
}

/**
 * Render a widget to React element
 */
export function renderWidget(
    widget: WidgetValue,
    ctx: RenderContext,
    key?: string
): React.ReactElement {
    const reactKey = key ?? widget.id;

    // Render children recursively
    const renderChildren = () =>
        widget.children.map((child, idx) =>
            renderWidget(child, ctx, `${widget.id}-${idx}`)
        );

    switch (widget.widgetType) {
        case 'Text':
            return (
                <Text
                    key={reactKey}
                    id={widget.id}
                    text={getStringProp(widget, 'text')}
                />
            );

        case 'Button':
            return (
                <Button
                    key={reactKey}
                    id={widget.id}
                    label={getStringProp(widget, 'label') || getStringProp(widget, 'text')}
                    disabled={widget.props.get('disabled') === true}
                    onClick={() => {
                        if (hasHandler(widget, 'click')) {
                            ctx.onEvent(widget.id, 'click');
                        }
                    }}
                />
            );

        case 'Row':
            return (
                <Row
                    key={reactKey}
                    id={widget.id}
                    gap={getNumberProp(widget, 'gap', 8)}
                >
                    {renderChildren()}
                </Row>
            );

        case 'Column':
            return (
                <Column
                    key={reactKey}
                    id={widget.id}
                    gap={getNumberProp(widget, 'gap', 8)}
                >
                    {renderChildren()}
                </Column>
            );

        case 'Container':
            return (
                <Container
                    key={reactKey}
                    id={widget.id}
                    padding={getNumberProp(widget, 'padding')}
                    margin={getNumberProp(widget, 'margin')}
                    backgroundColor={getStringProp(widget, 'backgroundColor') || undefined}
                    borderRadius={getNumberProp(widget, 'borderRadius')}
                >
                    {renderChildren()}
                </Container>
            );

        case 'Window':
            // Window is a root container
            return (
                <div
                    key={reactKey}
                    id={widget.id}
                    className="afns-window"
                    style={{
                        display: 'flex',
                        flexDirection: 'column',
                        minHeight: '100vh',
                        padding: '16px',
                    }}
                >
                    {getStringProp(widget, 'title') && (
                        <h1 className="afns-window-title" style={{ marginBottom: '16px' }}>
                            {getStringProp(widget, 'title')}
                        </h1>
                    )}
                    {renderChildren()}
                </div>
            );

        default:
            console.warn(`Unknown widget type: ${widget.widgetType}`);
            return (
                <div key={reactKey} id={widget.id} className="afns-unknown">
                    Unknown: {widget.widgetType}
                </div>
            );
    }
}

/**
 * Root renderer component
 */
export interface AfnsAppProps {
    root: WidgetValue | null;
    onEvent: (widgetId: string, eventType: string, payload?: unknown) => void;
}

export const AfnsApp: React.FC<AfnsAppProps> = ({ root, onEvent }) => {
    if (!root) {
        return <div className="afns-empty">No UI rendered</div>;
    }

    return renderWidget(root, { onEvent });
};
