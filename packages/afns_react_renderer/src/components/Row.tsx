import React from 'react';

interface RowProps {
    id: string;
    children: React.ReactNode;
    gap?: number;
    align?: 'start' | 'center' | 'end' | 'stretch';
    justify?: 'start' | 'center' | 'end' | 'between' | 'around';
}

export const Row: React.FC<RowProps> = ({
    id,
    children,
    gap = 8,
    align = 'center',
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
            className="afns-row"
            style={{
                display: 'flex',
                flexDirection: 'row',
                gap: `${gap}px`,
                alignItems: alignMap[align],
                justifyContent: justifyMap[justify],
            }}
        >
            {children}
        </div>
    );
};

Row.displayName = 'AFNS.Row';
