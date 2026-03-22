"use client";

import { memo } from "react";
import type { FeedbackData } from "@/lib/types";

export const FeedbackLoop = memo(function FeedbackLoop({ feedback }: { feedback: FeedbackData | null }) {
  const insights = feedback?.insights ?? [
    "Momentum signals performing well in trending regime",
    "Consider reducing position size in choppy markets",
  ];

  return (
    <div className="bg-bb-surface border border-bb-border h-full flex flex-col">
      <div className="px-2 py-1 border-b border-bb-border flex items-center justify-between">
        <span className="text-bb-amber font-bold text-[10px]">FEEDBACK</span>
        <span className="text-bb-green text-[9px]">● LEARNING</span>
      </div>
      <div className="text-[10px]">
        {insights.map((insight, i) => (
          <div key={i} className="px-2 py-1.5 border-b border-bb-border last:border-b-0">
            <span className="text-bb-cyan mr-1">&gt;</span>
            <span className="text-bb-white">{insight}</span>
          </div>
        ))}
      </div>
    </div>
  );
});
