import { useRef, useState, useEffect, type RefObject } from "react";
import type React from "react";

interface UsePanZoomOptions {
  /** When false the wheel-zoom handler is disabled (pan still works). */
  enabled: boolean;
}

interface UsePanZoomReturn {
  /** Attach this ref to the container element. */
  containerRef: RefObject<HTMLDivElement | null>;
  /** Current pan offset. */
  pan: { x: number; y: number };
  /** Current zoom level (1 = 100%). */
  zoom: number;
  /** Imperatively set pan. */
  setPan: React.Dispatch<React.SetStateAction<{ x: number; y: number }>>;
  /** Imperatively set zoom. */
  setZoom: React.Dispatch<React.SetStateAction<number>>;
  /** Spread these onto the container element. */
  pointerHandlers: {
    onPointerDown: (e: React.PointerEvent) => void;
    onPointerMove: (e: React.PointerEvent) => void;
    onPointerUp: (e: React.PointerEvent) => void;
    onPointerCancel: (e: React.PointerEvent) => void;
  };
  /** CSS style object to apply on the inner (transformed) element. */
  transformStyle: React.CSSProperties;
}

export function usePanZoom(options: UsePanZoomOptions): UsePanZoomReturn {
  const { enabled } = options;

  const containerRef = useRef<HTMLDivElement>(null);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [zoom, setZoom] = useState(1);
  const [isDragging, setIsDragging] = useState(false);
  const [lastPos, setLastPos] = useState({ x: 0, y: 0 });

  // Wheel-to-zoom
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const handleWheel = (e: WheelEvent) => {
      if (!enabled) return;

      e.preventDefault();
      e.stopPropagation();

      const zoomSensitivity = 0.001;
      const delta = -e.deltaY * zoomSensitivity;

      setZoom((prev) => {
        const next = prev * Math.exp(delta);
        // Clamp between 10% and 500%
        return Math.min(Math.max(next, 0.1), 5);
      });
    };

    container.addEventListener("wheel", handleWheel, { passive: false });
    return () => container.removeEventListener("wheel", handleWheel);
  }, [enabled]);

  // Pointer handlers for drag-to-pan
  const handlePointerDown = (e: React.PointerEvent) => {
    setIsDragging(true);
    setLastPos({ x: e.clientX, y: e.clientY });
    e.currentTarget.setPointerCapture(e.pointerId);
  };

  const handlePointerMove = (e: React.PointerEvent) => {
    if (!isDragging) return;
    const dx = e.clientX - lastPos.x;
    const dy = e.clientY - lastPos.y;
    setPan((prev) => ({ x: prev.x + dx, y: prev.y + dy }));
    setLastPos({ x: e.clientX, y: e.clientY });
  };

  const handlePointerUp = (e: React.PointerEvent) => {
    setIsDragging(false);
    if (e.currentTarget.hasPointerCapture(e.pointerId)) {
      e.currentTarget.releasePointerCapture(e.pointerId);
    }
  };

  const transformStyle: React.CSSProperties = {
    transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`,
    left: "50%",
    top: "50%",
    translate: "-50% -50%",
  };

  return {
    containerRef,
    pan,
    zoom,
    setPan,
    setZoom,
    pointerHandlers: {
      onPointerDown: handlePointerDown,
      onPointerMove: handlePointerMove,
      onPointerUp: handlePointerUp,
      onPointerCancel: handlePointerUp,
    },
    transformStyle,
  };
}
