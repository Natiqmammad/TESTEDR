import React from 'react';

interface ButtonProps {
    id: string;
    label: string;
    disabled?: boolean;
    onClick: () => void;
}

export const Button: React.FC<ButtonProps> = ({ id, label, disabled, onClick }) => {
    return (
        <button
            id={id}
            className="afns-button"
            disabled={disabled}
            onClick={onClick}
        >
            {label}
        </button>
    );
};

Button.displayName = 'AFNS.Button';
