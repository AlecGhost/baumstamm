import React from "react";
import type { Connections } from "@/lib/types";

interface ConnectionCellProps {
    connections: Connections;
}

export const ConnectionCell: React.FC<ConnectionCellProps> = ({ connections }) => {
    const { orientation, passing, ending, crossing } = connections;
    const isUp = orientation === "Up";

    return (
        <div className="w-full h-full min-h-[40px] relative">
            <svg
                className="absolute inset-0 w-full h-full pointer-events-none"
                preserveAspectRatio="none"
            >
                {/* Passing: horizontal line across the entire cell */}
                {passing.map((p, i) => {
                    const y = (p.y_fraction.numerator / p.y_fraction.denominator) * 100;
                    return (
                        <line
                            key={`pass-${i}`}
                            x1="0%"
                            y1={`${y}%`}
                            x2="100%"
                            y2={`${y}%`}
                            stroke="currentColor"
                            strokeWidth="2"
                            className="text-muted-foreground"
                        />
                    );
                })}

                {/* Ending: horizontal line from origin to x, and vertical line to the edge */}
                {ending.map((e, i) => {
                    const x = (e.x_fraction.numerator / e.x_fraction.denominator) * 100;
                    const y = (e.y_fraction.numerator / e.y_fraction.denominator) * 100;
                    const edgeY = isUp ? "0%" : "100%";

                    return (
                        <g key={`end-${i}`} className="text-muted-foreground" stroke="currentColor" strokeWidth="2" fill="none">
                            {/* Horizontal part */}
                            {e.origin === "Left" && (
                                <line x1="0%" y1={`${y}%`} x2={`${x}%`} y2={`${y}%`} />
                            )}
                            {e.origin === "Right" && (
                                <line x1={`${x}%`} y1={`${y}%`} x2="100%" y2={`${y}%`} />
                            )}
                            {/* Vertical part */}
                            <line x1={`${x}%`} y1={`${y}%`} x2={`${x}%`} y2={edgeY} />

                            {/* Corner rounding (optional polish) */}
                            {e.origin !== "None" && (
                                <circle cx={`${x}%`} cy={`${y}%`} r="2" fill="currentColor" stroke="none" />
                            )}
                        </g>
                    );
                })}

                {/* Crossing: horizontal line from origin to x, vertical line connecting horizontal line to opposite edge */}
                {crossing.map((c, i) => {
                    const x = (c.x_fraction.numerator / c.x_fraction.denominator) * 100;
                    const y = (c.y_fraction.numerator / c.y_fraction.denominator) * 100;
                    
                    const startY = isUp ? `${y}%` : "0%";
                    const endY = isUp ? "100%" : `${y}%`;

                    return (
                        <g key={`cross-${i}`} className="text-muted-foreground" stroke="currentColor" strokeWidth="2" fill="none">
                            {/* Horizontal part */}
                            {c.origin === "Left" && (
                                <line x1="0%" y1={`${y}%`} x2={`${x}%`} y2={`${y}%`} />
                            )}
                            {c.origin === "Right" && (
                                <line x1={`${x}%`} y1={`${y}%`} x2="100%" y2={`${y}%`} />
                            )}
                            {/* Vertical trunk to opposite edge */}
                            <line x1={`${x}%`} y1={startY} x2={`${x}%`} y2={endY} />

                            {/* Corner dot */}
                            {c.origin !== "None" && (
                                <circle cx={`${x}%`} cy={`${y}%`} r="2" fill="currentColor" stroke="none" />
                            )}
                        </g>
                    );
                })}
            </svg>
        </div>
    );
};
