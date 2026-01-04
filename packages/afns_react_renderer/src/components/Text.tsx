import React from 'react';

interface TextProps {
    id: string;
    text: string;
}

export const Text: React.FC<TextProps> = ({ id, text }) => {
    return (
        <span id={id} className="afns-text">
            {text}
        </span>
    );
};

Text.displayName = 'AFNS.Text';
