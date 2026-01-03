import React from 'react';
import { WidgetNode } from '../types';

interface TextWidgetProps {
    node: WidgetNode;
}

/**
 * Text widget - renders a span with the text content
 */
function TextWidget({ node }: TextWidgetProps) {
    const text = node.props.text || '';

    return (
        <span style={styles.text}>
            {text}
        </span>
    );
}

const styles: Record<string, React.CSSProperties> = {
    text: {
        fontSize: '16px',
        color: '#e0e0e0',
        lineHeight: 1.5,
        display: 'block',
        padding: '5px 0',
    },
};

export default TextWidget;
