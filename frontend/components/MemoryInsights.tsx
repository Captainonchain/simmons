"use client";

import { memo, useState } from "react";

interface AgentStats {
  totalPredictions: number;
  correctPredictions: number;
  accuracy: number;
}

interface MemoryInsightsProps {
  totalLearnings?: number;
  totalReflections?: number;
  agentStats?: Record<string, AgentStats>;
  recentLessons?: string[];
  winningPatterns?: string[];
  avoidPatterns?: string[];
}

const MOCK_STATS: Record<string, AgentStats> = {
  Technical: { totalPredictions: 45, correctPredictions: 32, accuracy: 0.71 },
  Sentiment: { totalPredictions: 38, correctPredictions: 29, accuracy: 0.76 },
  "On-chain": { totalPredictions: 42, correctPredictions: 35, accuracy: 0.83 },
  Fundamental: { totalPredictions: 30, correctPredictions: 18, accuracy: 0.60 },
};

const MOCK_LESSONS = [
  "RSI < 30 + volume spike → high reversal probability",
  "Whale accumulation confirms technical signals",
  "Reduce size when MACD not confirmed by volume",
  "Avoid trading first 15min after regime change",
];

const MOCK_WINNING = [
  "Oversold RSI + smart money buying",
  "Volume spike at support level",
  "Multi-agent consensus > 80%",
];

const MOCK_AVOID = [
  "Trading against whale distribution",
  "Entering during choppy/ranging regime",
  "Single-agent signals without confirmation",
];

export const MemoryInsights = memo(function MemoryInsights({
  totalLearnings = 24,
  totalReflections = 12,
  agentStats = MOCK_STATS,
  recentLessons = MOCK_LESSONS,
  winningPatterns = MOCK_WINNING,
  avoidPatterns = MOCK_AVOID,
}: MemoryInsightsProps) {
  const [tab, setTab] = useState<"stats" | "lessons" | "patterns">("stats");

  const sortedAgents = Object.entries(agentStats).sort((a, b) => b[1].accuracy - a[1].accuracy);

  const getAccuracyColor = (acc: number) => {
    if (acc >= 0.7) return "text-bb-green";
    if (acc >= 0.5) return "text-bb-amber";
    return "text-bb-red";
  };

  return (
    <div className="grid-cell h-full">
      <div className="grid-cell-header">
        <div className="flex items-center gap-2">
          <span className="grid-cell-title">MEMORY</span>
          <span className="text-[8px] text-bb-dim">REFLECT</span>
        </div>
        <div className="flex items-center gap-2 text-[8px]">
          <span className="text-bb-cyan">{totalLearnings}</span>
          <span className="text-bb-muted">/</span>
          <span className="text-bb-dim">{totalReflections}</span>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex border-b border-bb-border shrink-0">
        {(["stats", "lessons", "patterns"] as const).map((t) => (
          <button
            key={t}
            onClick={() => setTab(t)}
            className={`flex-1 py-1.5 text-[8px] uppercase transition-all relative ${
              tab === t ? "text-bb-orange bg-bb-raised" : "text-bb-dim hover:text-bb-white hover:bg-bb-raised/50"
            }`}
          >
            {t}
            {tab === t && <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-bb-orange" />}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-auto p-2">
        {tab === "stats" && (
          <div className="space-y-1.5">
            {sortedAgents.map(([name, stats]) => (
              <div key={name} className="bg-bb-raised border border-bb-border p-2 hover:border-bb-border-light transition-colors">
                <div className="flex items-center justify-between mb-1">
                  <span className="text-bb-cyan text-[9px] font-medium">{name}</span>
                  <span className={`text-[11px] font-bold stat-value ${getAccuracyColor(stats.accuracy)}`}>
                    {(stats.accuracy * 100).toFixed(0)}%
                  </span>
                </div>
                <div className="flex items-center justify-between gap-2">
                  <span className="text-[8px] text-bb-dim">
                    {stats.correctPredictions}/{stats.totalPredictions} correct
                  </span>
                  <div className="w-16 h-1.5 bg-bb-border overflow-hidden">
                    <div
                      className={`h-full transition-all ${getAccuracyColor(stats.accuracy).replace('text-', 'bg-')}`}
                      style={{ width: `${stats.accuracy * 100}%` }}
                    />
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}

        {tab === "lessons" && (
          <div className="space-y-1.5">
            {recentLessons.map((lesson, i) => (
              <div key={i} className="bg-bb-raised border border-bb-border p-2 text-[9px] hover:border-bb-blue/30 transition-colors">
                <span className="text-bb-blue mr-2">→</span>
                <span className="text-bb-white">{lesson}</span>
              </div>
            ))}
          </div>
        )}

        {tab === "patterns" && (
          <div className="space-y-3">
            {/* Winning Patterns */}
            <div>
              <div className="text-[8px] text-bb-green uppercase tracking-wider mb-1.5 font-medium">
                ✓ Winning Patterns
              </div>
              <div className="space-y-1">
                {winningPatterns.map((p, i) => (
                  <div key={i} className="bg-bb-raised border border-bb-green/20 p-1.5 text-[8px] hover:border-bb-green/40 transition-colors">
                    <span className="text-bb-white">{p}</span>
                  </div>
                ))}
              </div>
            </div>

            {/* Avoid Patterns */}
            <div>
              <div className="text-[8px] text-bb-red uppercase tracking-wider mb-1.5 font-medium">
                ✗ Avoid Patterns
              </div>
              <div className="space-y-1">
                {avoidPatterns.map((p, i) => (
                  <div key={i} className="bg-bb-raised border border-bb-red/20 p-1.5 text-[8px] hover:border-bb-red/40 transition-colors">
                    <span className="text-bb-white">{p}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
});
