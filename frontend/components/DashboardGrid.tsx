"use client";

import { useState, useCallback, useRef, useEffect, memo } from "react";
import { ResponsiveGridLayout, verticalCompactor, type Layout, type ResponsiveLayouts } from "react-grid-layout";
import "react-grid-layout/css/styles.css";

const STORAGE_KEY = "simmons-grid-v3";

// Layout reflects Dual Brain v3.0 architecture flow:
// Stats → Chart + Markets + TA Brain (side) → Fund Brain (side)
// → Consensus → Orchestrator → Execution
// → Portfolio + Risk/Kelly → Signals + Feedback
const DEFAULT_LAYOUTS: ResponsiveLayouts = {
  xl: [
    // Row 0: Stats overview
    { i: "stats",        x: 0,  y: 0,  w: 12, h: 2,  isResizable: false },
    // Row 1: Data layer — Chart, Markets, TA Brain
    { i: "chart",        x: 0,  y: 2,  w: 4,  h: 8 },
    { i: "markets",      x: 4,  y: 2,  w: 4,  h: 8 },
    { i: "ta-brain",     x: 8,  y: 2,  w: 4,  h: 12 },
    // Row 2: Dual Brain — Fund Brain + Consensus + Orchestrator
    { i: "fund-brain",   x: 0,  y: 10, w: 4,  h: 12 },
    { i: "consensus",    x: 4,  y: 10, w: 4,  h: 8 },
    { i: "orchestrator", x: 4,  y: 18, w: 4,  h: 10 },
    { i: "execution",    x: 8,  y: 14, w: 4,  h: 8 },
    // Row 3: Portfolio + Risk
    { i: "portfolio",    x: 0,  y: 22, w: 6,  h: 6 },
    { i: "risk",         x: 6,  y: 22, w: 3,  h: 6 },
    { i: "kelly",        x: 9,  y: 22, w: 3,  h: 6 },
    // Row 4: Signals + Feedback
    { i: "signals",      x: 0,  y: 28, w: 6,  h: 6 },
    { i: "feedback",     x: 6,  y: 28, w: 6,  h: 4 },
  ],
  md: [
    { i: "stats",        x: 0, y: 0,  w: 6, h: 2,  isResizable: false },
    { i: "chart",        x: 0, y: 2,  w: 3, h: 7 },
    { i: "markets",      x: 3, y: 2,  w: 3, h: 7 },
    { i: "ta-brain",     x: 0, y: 9,  w: 3, h: 12 },
    { i: "fund-brain",   x: 3, y: 9,  w: 3, h: 12 },
    { i: "consensus",    x: 0, y: 21, w: 3, h: 8 },
    { i: "orchestrator", x: 3, y: 21, w: 3, h: 10 },
    { i: "execution",    x: 0, y: 29, w: 3, h: 8 },
    { i: "portfolio",    x: 3, y: 31, w: 3, h: 6 },
    { i: "risk",         x: 0, y: 37, w: 3, h: 6 },
    { i: "kelly",        x: 3, y: 37, w: 3, h: 6 },
    { i: "signals",      x: 0, y: 43, w: 6, h: 6 },
    { i: "feedback",     x: 0, y: 49, w: 6, h: 4 },
  ],
  sm: [
    { i: "stats",        x: 0, y: 0,  w: 2, h: 5,  isResizable: false },
    { i: "chart",        x: 0, y: 5,  w: 2, h: 7 },
    { i: "ta-brain",     x: 0, y: 12, w: 2, h: 12 },
    { i: "fund-brain",   x: 0, y: 24, w: 2, h: 12 },
    { i: "consensus",    x: 0, y: 36, w: 2, h: 8 },
    { i: "orchestrator", x: 0, y: 44, w: 2, h: 10 },
    { i: "execution",    x: 0, y: 54, w: 2, h: 8 },
    { i: "portfolio",    x: 0, y: 62, w: 2, h: 6 },
    { i: "markets",      x: 0, y: 68, w: 2, h: 7 },
    { i: "risk",         x: 0, y: 75, w: 2, h: 5 },
    { i: "kelly",        x: 0, y: 80, w: 2, h: 5 },
    { i: "signals",      x: 0, y: 85, w: 2, h: 6 },
    { i: "feedback",     x: 0, y: 91, w: 2, h: 4 },
  ],
};

function loadLayouts(): ResponsiveLayouts | null {
  if (typeof window === "undefined") return null;
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    return saved ? JSON.parse(saved) : null;
  } catch {
    return null;
  }
}

function saveLayouts(layouts: ResponsiveLayouts) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(layouts));
  } catch {}
}

interface DashboardGridProps {
  children: Record<string, React.ReactNode>;
  editMode: boolean;
}

export const DashboardGrid = memo(function DashboardGrid({ children, editMode }: DashboardGridProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  const [layouts, setLayouts] = useState<ResponsiveLayouts>(() => loadLayouts() ?? DEFAULT_LAYOUTS);

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

  const handleLayoutChange = useCallback((_layout: Layout, allLayouts: ResponsiveLayouts) => {
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
          margin={[3, 3]}
          containerPadding={[3, 3]}
          dragConfig={{ enabled: editMode, handle: ".grid-drag-handle", bounded: false, threshold: 3 }}
          resizeConfig={{ enabled: editMode, handles: ["se"] }}
          onLayoutChange={handleLayoutChange}
          compactor={verticalCompactor}
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
});
