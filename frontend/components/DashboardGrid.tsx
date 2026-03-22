"use client";

import { useState, useCallback, useRef, useEffect } from "react";
import { ResponsiveGridLayout, type Layout, type Layouts } from "react-grid-layout";
import "react-grid-layout/css/styles.css";

const STORAGE_KEY = "simmons-grid-layouts";

const DEFAULT_LAYOUTS: Layouts = {
  xl: [
    { i: "stats",     x: 0, y: 0,  w: 12, h: 2,  isResizable: false },
    { i: "chart",     x: 0, y: 2,  w: 4,  h: 7 },
    { i: "markets",   x: 4, y: 2,  w: 4,  h: 7 },
    { i: "signals",   x: 8, y: 2,  w: 4,  h: 7 },
    { i: "portfolio", x: 0, y: 9,  w: 8,  h: 6 },
    { i: "brain",     x: 8, y: 9,  w: 4,  h: 6 },
    { i: "nunchi",    x: 0, y: 15, w: 3,  h: 6 },
    { i: "risk",      x: 3, y: 15, w: 3,  h: 6 },
    { i: "kelly",     x: 6, y: 15, w: 3,  h: 6 },
    { i: "mev",       x: 9, y: 15, w: 3,  h: 6 },
    { i: "infra",     x: 0, y: 21, w: 4,  h: 4 },
    { i: "feedback",  x: 4, y: 21, w: 4,  h: 4 },
    { i: "headlines", x: 8, y: 21, w: 4,  h: 6 },
  ],
  md: [
    { i: "stats",     x: 0, y: 0,  w: 6, h: 2,  isResizable: false },
    { i: "chart",     x: 0, y: 2,  w: 3, h: 7 },
    { i: "markets",   x: 3, y: 2,  w: 3, h: 7 },
    { i: "signals",   x: 0, y: 9,  w: 6, h: 5 },
    { i: "portfolio", x: 0, y: 14, w: 6, h: 6 },
    { i: "brain",     x: 0, y: 20, w: 3, h: 5 },
    { i: "nunchi",    x: 3, y: 20, w: 3, h: 5 },
    { i: "risk",      x: 0, y: 25, w: 3, h: 6 },
    { i: "kelly",     x: 3, y: 25, w: 3, h: 6 },
    { i: "mev",       x: 0, y: 31, w: 3, h: 4 },
    { i: "infra",     x: 3, y: 31, w: 3, h: 4 },
    { i: "feedback",  x: 0, y: 35, w: 3, h: 4 },
    { i: "headlines", x: 3, y: 35, w: 3, h: 6 },
  ],
  sm: [
    { i: "stats",     x: 0, y: 0,  w: 2, h: 4,  isResizable: false },
    { i: "chart",     x: 0, y: 4,  w: 2, h: 7 },
    { i: "portfolio", x: 0, y: 11, w: 2, h: 6 },
    { i: "markets",   x: 0, y: 17, w: 2, h: 7 },
    { i: "signals",   x: 0, y: 24, w: 2, h: 6 },
    { i: "brain",     x: 0, y: 30, w: 2, h: 5 },
    { i: "nunchi",    x: 0, y: 35, w: 1, h: 6 },
    { i: "risk",      x: 1, y: 35, w: 1, h: 6 },
    { i: "kelly",     x: 0, y: 41, w: 1, h: 5 },
    { i: "mev",       x: 1, y: 41, w: 1, h: 4 },
    { i: "infra",     x: 0, y: 46, w: 2, h: 3 },
    { i: "feedback",  x: 0, y: 49, w: 2, h: 4 },
    { i: "headlines", x: 0, y: 53, w: 2, h: 6 },
  ],
};

function loadLayouts(): Layouts | null {
  if (typeof window === "undefined") return null;
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    return saved ? JSON.parse(saved) : null;
  } catch {
    return null;
  }
}

function saveLayouts(layouts: Layouts) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(layouts));
  } catch {}
}

interface DashboardGridProps {
  children: Record<string, React.ReactNode>;
  editMode: boolean;
}

export function DashboardGrid({ children, editMode }: DashboardGridProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  const [layouts, setLayouts] = useState<Layouts>(() => loadLayouts() ?? DEFAULT_LAYOUTS);

  // Measure container width on mount + resize
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    const measure = () => setWidth(el.offsetWidth);
    measure();

    const ro = new ResizeObserver(() => measure());
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const handleLayoutChange = useCallback((_: Layout[], allLayouts: Layouts) => {
    setLayouts(allLayouts);
    saveLayouts(allLayouts);
  }, []);

  const handleReset = useCallback(() => {
    setLayouts(DEFAULT_LAYOUTS);
    saveLayouts(DEFAULT_LAYOUTS);
  }, []);

  const panelKeys = Object.keys(children);

  return (
    <div ref={containerRef} className="relative">
      {editMode && (
        <div className="sticky top-0 z-30 bg-bb-panel border-b border-bb-border px-3 py-1 flex items-center justify-between text-[10px]">
          <span className="text-bb-orange font-bold">EDIT MODE — DRAG PANELS TO REARRANGE</span>
          <button onClick={handleReset} className="text-bb-red hover:text-bb-bright transition-colors font-bold">
            RESET LAYOUT
          </button>
        </div>
      )}
      {width > 0 ? (
        <ResponsiveGridLayout
          width={width}
          layouts={layouts}
          breakpoints={{ xl: 1280, md: 768, sm: 0 }}
          cols={{ xl: 12, md: 6, sm: 2 }}
          rowHeight={35}
          margin={[1, 1]}
          containerPadding={[1, 1]}
          isDraggable={editMode}
          isResizable={editMode}
          draggableHandle=".grid-drag-handle"
          onLayoutChange={handleLayoutChange}
          compactType="vertical"
          useCSSTransforms
        >
          {panelKeys.map((key) => (
            <div key={key} className="overflow-hidden">
              {editMode && (
                <div className="grid-drag-handle absolute top-0 left-0 right-0 h-6 z-20 cursor-grab active:cursor-grabbing bg-bb-orange/10 border-b border-bb-orange/30 flex items-center justify-center">
                  <span className="text-[8px] text-bb-orange font-bold tracking-widest select-none">⠿ DRAG</span>
                </div>
              )}
              <div className={`h-full ${editMode ? "ring-1 ring-bb-orange/20" : ""}`}>
                {children[key]}
              </div>
            </div>
          ))}
        </ResponsiveGridLayout>
      ) : (
        /* Fallback: render panels stacked while measuring */
        <div className="flex flex-col gap-px p-px">
          {panelKeys.map((key) => (
            <div key={key}>{children[key]}</div>
          ))}
        </div>
      )}
    </div>
  );
}
