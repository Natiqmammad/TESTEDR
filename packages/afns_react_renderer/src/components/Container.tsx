import React from 'react';

interface ContainerProps {
    id: string;
    children: React.ReactNode;
    padding?: number;
    margin?: number;
    backgroundColor?: string;
    borderRadius?: number;
    border?: string;
    width?: string | number;
    height?: string | number;
}

export const Container: React.FC<ContainerProps> = ({
    id,
    children,
    padding = 0,
    margin = 0,
    backgroundColor,
    borderRadius = 0,
    border,
    width,
    height,
}) => {
    return (
        <div
            id={id}
            className="afns-container"
            style={{
                padding: `${padding}px`,
                margin: `${margin}px`,
                backgroundColor,
                borderRadius: `${borderRadius}px`,
                border,
                width: typeof width === 'number' ? `${width}px` : width,
                height: typeof height === 'number' ? `${height}px` : height,
                boxSizing: 'border-box',
            }}
        >
            {children}
        </div>
    );
};

Container.displayName = 'AFNS.Container';
