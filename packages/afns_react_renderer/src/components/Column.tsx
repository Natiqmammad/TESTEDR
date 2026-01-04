import React from 'react';

interface ColumnProps {
    id: string;
    children: React.ReactNode;
    gap?: number;
    align?: 'start' | 'center' | 'end' | 'stretch';
    justify?: 'start' | 'center' | 'end' | 'between' | 'around';
}

export const Column: React.FC<ColumnProps> = ({
    id,
    children,
    gap = 8,
    align = 'stretch',
    justify = 'start',
}) => {
    const justifyMap = {
        start: 'flex-start',
        center: 'center',
        end: 'flex-end',
        between: 'space-between',
        around: 'space-around',
    };

    const alignMap = {
        start: 'flex-start',
        center: 'center',
        end: 'flex-end',
        stretch: 'stretch',
    };

    return (
        <div
            id={id}
            className="afns-column"
            style={{
                display: 'flex',
                flexDirection: 'column',
                gap: `${gap}px`,
                alignItems: alignMap[align],
                justifyContent: justifyMap[justify],
            }}
        >
            {children}
        </div>
    );
};

Column.displayName = 'AFNS.Column';
