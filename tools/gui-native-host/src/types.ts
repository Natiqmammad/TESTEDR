/**
 * Widget type definitions matching the JSON schema from the runtime
 */

export interface WidgetNode {
    id: number;
    type: string;
    props: Record<string, string>;
    children: WidgetNode[];
}

export interface RenderMessage {
    kind: 'render';
    tree: WidgetNode;
}

export interface EventMessage {
    kind: 'event';
    event: string;
    target: string;
    handler: string;
}

export type Message = RenderMessage | EventMessage;

/**
 * Parse a JSON line into a Message
 */
export function parseMessage(line: string): Message | null {
    try {
        const data = JSON.parse(line.trim());
        if (data.kind === 'render' && data.tree) {
            return data as RenderMessage;
        }
        return null;
    } catch {
        return null;
    }
}

/**
 * Create an event message to send to the runtime
 */
export function createEventMessage(
    event: string,
    targetId: number,
    handler: string
): EventMessage {
    return {
        kind: 'event',
        event,
        target: String(targetId),
        handler,
    };
}
