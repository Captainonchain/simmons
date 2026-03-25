"use client";

import { useState } from "react";

interface AgentStats {
  total_predictions: number;
  correct_predictions: number;
  accuracy: number;
}

interface MemoryProps {
  totalLearnings?: number;
  totalReflections?: number;
  agentStats?: Record<string, AgentStats>;
  recentLessons?: string[];
  avoidPatterns?: string[];
  winningPatterns?: string[];
}

export function MemoryInsights({
  totalLearnings = 0,
  totalReflections = 0,
  agentStats = {},
  recentLessons = [],
  avoidPatterns = [],
  winningPatterns = [],
}: MemoryProps) {
  const [activeTab, setActiveTab] = useState<"stats" | "lessons" | "patterns">("stats");

  const sortedAgents = Object.entries(agentStats)
    .sort((a, b) => b[1].accuracy - a[1].accuracy);

  const getAccuracyColor = (accuracy: number) => {
    if (accuracy >= 0.7) return "text-bb-green";
    if (accuracy >= 0.5) return "text-bb-amber";
    return "text-bb-red";
  };

  return (
    <div className="bg-bb-surface border border-bb-border h-full flex flex-col">
      <div className="bg-bb-panel px-2 py-1 border-b border-bb-border flex items-center justify-between">
        <span className="text-[10px] text-bb-orange font-semibold tracking-wider">MEMORY & LEARNING</span>
        <div className="flex items-center gap-1 text-[9px]">
          <span className="text-bb-dim">{totalLearnings} learnings</span>
          <span className="text-bb-border">|</span>
          <span className="text-bb-dim">{totalReflections} reflections</span>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex border-b border-bb-border text-[9px]">
        {[
          { id: "stats", label: "AGENT STATS" },
          { id: "lessons", label: "LESSONS" },
          { id: "patterns", label: "PATTERNS" },
        ].map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id as typeof activeTab)}
            className={`flex-1 px-2 py-1 transition-colors ${
              activeTab === tab.id
                ? "bg-bb-raised text-bb-orange border-b border-bb-orange"
                : "text-bb-dim hover:text-bb-white"
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-y-auto p-2 text-[10px]">
        {activeTab === "stats" && (
          <div className="space-y-1">
            {sortedAgents.length === 0 ? (
              <div className="text-bb-dim text-center py-4">No agent stats yet</div>
            ) : (
              sortedAgents.map(([name, stats]) => (
                <div key={name} className="bg-bb-raised border border-bb-border p-1.5">
                  <div className="flex items-center justify-between">
                    <span className="text-bb-cyan capitalize">
                      {name.replace(/_/g, " ")}
                    </span>
                    <span className={getAccuracyColor(stats.accuracy)}>
                      {(stats.accuracy * 100).toFixed(0)}%
                    </span>
                  </div>
                  <div className="flex items-center justify-between text-[9px] text-bb-dim mt-0.5">
                    <span>{stats.correct_predictions}/{stats.total_predictions} correct</span>
                    <div className="h-1 w-16 bg-bb-border">
                      <div
                        className={`h-full ${stats.accuracy >= 0.5 ? "bg-bb-green" : "bg-bb-red"}`}
                        style={{ width: `${stats.accuracy * 100}%` }}
                      />
                    </div>
                  </div>
                </div>
              ))
            )}
          </div>
        )}

        {activeTab === "lessons" && (
          <div className="space-y-1">
            {recentLessons.length === 0 ? (
              <div className="text-bb-dim text-center py-4">No lessons recorded</div>
            ) : (
              recentLessons.map((lesson, i) => (
                <div key={i} className="bg-bb-raised border border-bb-border p-1.5">
                  <div className="flex items-start gap-1.5">
                    <span className="text-bb-blue">•</span>
                    <span className="text-bb-white">{lesson}</span>
                  </div>
                </div>
              ))
            )}
          </div>
        )}

        {activeTab === "patterns" && (
          <div className="space-y-2">
            {/* Winning Patterns */}
            <div>
              <div className="text-bb-green text-[9px] uppercase tracking-wider mb-1">
                Winning Patterns
              </div>
              {winningPatterns.length === 0 ? (
                <div className="text-bb-dim text-[9px]">None recorded</div>
              ) : (
                <div className="space-y-1">
                  {winningPatterns.map((pattern, i) => (
                    <div key={i} className="bg-bb-raised border border-bb-green/30 p-1.5">
                      <span className="text-bb-green">✓</span>{" "}
                      <span className="text-bb-white">{pattern}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>

            {/* Avoid Patterns */}
            <div>
              <div className="text-bb-red text-[9px] uppercase tracking-wider mb-1">
                Avoid Patterns
              </div>
              {avoidPatterns.length === 0 ? (
                <div className="text-bb-dim text-[9px]">None recorded</div>
              ) : (
                <div className="space-y-1">
                  {avoidPatterns.map((pattern, i) => (
                    <div key={i} className="bg-bb-raised border border-bb-red/30 p-1.5">
                      <span className="text-bb-red">✗</span>{" "}
                      <span className="text-bb-white">{pattern}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
