import React, { useState } from 'react';
import { WidgetNode } from '../types';
import { EventHandler } from './index';

interface ButtonWidgetProps {
    node: WidgetNode;
    onEvent: EventHandler;
}

/**
 * Button widget - renders an interactive button that sends click events
 */
function ButtonWidget({ node, onEvent }: ButtonWidgetProps) {
    const [isPressed, setIsPressed] = useState(false);
    const [isHovered, setIsHovered] = useState(false);

    const label = node.props.label || 'Button';
    const handler = node.props.handler || '';

    const handleClick = () => {
        if (handler) {
            onEvent(node.id, 'click', handler);
        }
    };

    const style: React.CSSProperties = {
        ...styles.button,
        ...(isHovered ? styles.buttonHover : {}),
        ...(isPressed ? styles.buttonPressed : {}),
    };

    return (
        <button
            style={style}
            onClick={handleClick}
            onMouseEnter={() => setIsHovered(true)}
            onMouseLeave={() => {
                setIsHovered(false);
                setIsPressed(false);
            }}
            onMouseDown={() => setIsPressed(true)}
            onMouseUp={() => setIsPressed(false)}
        >
            {label}
        </button>
    );
}

const styles: Record<string, React.CSSProperties> = {
    button: {
        padding: '12px 24px',
        fontSize: '14px',
        fontWeight: 500,
        color: '#fff',
        background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
        border: 'none',
        borderRadius: '8px',
        cursor: 'pointer',
        transition: 'all 0.2s ease',
        boxShadow: '0 4px 15px rgba(102, 126, 234, 0.3)',
        margin: '5px',
    },
    buttonHover: {
        transform: 'translateY(-2px)',
        boxShadow: '0 6px 20px rgba(102, 126, 234, 0.4)',
    },
    buttonPressed: {
        transform: 'translateY(0)',
        boxShadow: '0 2px 10px rgba(102, 126, 234, 0.3)',
    },
};

export default ButtonWidget;
