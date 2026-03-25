"use client";

import { useState } from "react";

interface AgentVote {
  agent: string;
  recommendation: string;
  confidence: number;
  reason: string;
}

interface DebateProps {
  bullConviction?: number;
  bearConviction?: number;
  finalDecision?: string;
  agentVotes?: AgentVote[];
}

export function AgentDebate({ bullConviction = 0, bearConviction = 0, finalDecision, agentVotes = [] }: DebateProps) {
  const [expanded, setExpanded] = useState(false);

  const getRecommendationColor = (rec: string) => {
    const r = rec.toLowerCase();
    if (r.includes("buy") || r.includes("long")) return "text-bb-green";
    if (r.includes("sell") || r.includes("short")) return "text-bb-red";
    return "text-bb-amber";
  };

  const getConfidenceBar = (confidence: number) => {
    const width = Math.min(100, Math.max(0, confidence * 100));
    return (
      <div className="h-1 bg-bb-border w-full mt-1">
        <div
          className="h-full bg-bb-cyan transition-all"
          style={{ width: `${width}%` }}
        />
      </div>
    );
  };

  // Group votes by type
  const analysts = agentVotes.filter(v =>
    ["technical", "fundamental", "sentiment", "onchain"].some(t => v.agent.toLowerCase().includes(t))
  );
  const researchers = agentVotes.filter(v =>
    ["bull", "bear", "research"].some(t => v.agent.toLowerCase().includes(t))
  );
  const riskManagers = agentVotes.filter(v =>
    ["risk", "aggressive", "conservative", "neutral"].some(t => v.agent.toLowerCase().includes(t))
  );

  return (
    <div className="bg-bb-surface border border-bb-border h-full flex flex-col">
      <div
        className="bg-bb-panel px-2 py-1 border-b border-bb-border flex items-center justify-between cursor-pointer select-none"
        onClick={() => setExpanded(!expanded)}
      >
        <span className="text-[10px] text-bb-orange font-semibold tracking-wider">AGENT DEBATE</span>
        <span className="text-[9px] text-bb-dim">{expanded ? "▼" : "▶"}</span>
      </div>

      <div className="flex-1 overflow-y-auto p-2 space-y-2 text-[10px]">
        {/* Bull vs Bear Summary */}
        <div className="grid grid-cols-2 gap-2">
          <div className="bg-bb-raised border border-bb-border p-2">
            <div className="flex items-center justify-between">
              <span className="text-bb-green font-bold">BULL</span>
              <span className="text-bb-green">{(bullConviction * 100).toFixed(0)}%</span>
            </div>
            <div className="h-1.5 bg-bb-border mt-1">
              <div
                className="h-full bg-bb-green transition-all"
                style={{ width: `${bullConviction * 100}%` }}
              />
            </div>
          </div>
          <div className="bg-bb-raised border border-bb-border p-2">
            <div className="flex items-center justify-between">
              <span className="text-bb-red font-bold">BEAR</span>
              <span className="text-bb-red">{(bearConviction * 100).toFixed(0)}%</span>
            </div>
            <div className="h-1.5 bg-bb-border mt-1">
              <div
                className="h-full bg-bb-red transition-all"
                style={{ width: `${bearConviction * 100}%` }}
              />
            </div>
          </div>
        </div>

        {/* Final Decision */}
        {finalDecision && (
          <div className="bg-bb-panel border border-bb-orange/30 p-2">
            <div className="flex items-center gap-2">
              <span className="text-bb-orange">DECISION:</span>
              <span className={`font-bold ${getRecommendationColor(finalDecision)}`}>
                {finalDecision.toUpperCase()}
              </span>
            </div>
          </div>
        )}

        {expanded && (
          <>
            {/* Analysts */}
            {analysts.length > 0 && (
              <div className="space-y-1">
                <div className="text-bb-dim text-[9px] uppercase tracking-wider">Analysts</div>
                {analysts.map((vote, i) => (
                  <div key={i} className="bg-bb-raised border border-bb-border p-1.5">
                    <div className="flex items-center justify-between">
                      <span className="text-bb-cyan">{vote.agent}</span>
                      <span className={getRecommendationColor(vote.recommendation)}>
                        {vote.recommendation}
                      </span>
                    </div>
                    {getConfidenceBar(vote.confidence)}
                    <div className="text-bb-dim text-[9px] mt-1 truncate">{vote.reason}</div>
                  </div>
                ))}
              </div>
            )}

            {/* Researchers */}
            {researchers.length > 0 && (
              <div className="space-y-1">
                <div className="text-bb-dim text-[9px] uppercase tracking-wider">Researchers</div>
                {researchers.map((vote, i) => (
                  <div key={i} className="bg-bb-raised border border-bb-border p-1.5">
                    <div className="flex items-center justify-between">
                      <span className="text-bb-magenta">{vote.agent}</span>
                      <span className={getRecommendationColor(vote.recommendation)}>
                        {vote.recommendation}
                      </span>
                    </div>
                    {getConfidenceBar(vote.confidence)}
                    <div className="text-bb-dim text-[9px] mt-1 truncate">{vote.reason}</div>
                  </div>
                ))}
              </div>
            )}

            {/* Risk Managers */}
            {riskManagers.length > 0 && (
              <div className="space-y-1">
                <div className="text-bb-dim text-[9px] uppercase tracking-wider">Risk Team</div>
                {riskManagers.map((vote, i) => (
                  <div key={i} className="bg-bb-raised border border-bb-border p-1.5">
                    <div className="flex items-center justify-between">
                      <span className="text-bb-amber">{vote.agent}</span>
                      <span className="text-bb-white">{vote.recommendation}</span>
                    </div>
                    {getConfidenceBar(vote.confidence)}
                    <div className="text-bb-dim text-[9px] mt-1 truncate">{vote.reason}</div>
                  </div>
                ))}
              </div>
            )}
          </>
        )}

        {!expanded && agentVotes.length > 0 && (
          <div className="text-bb-dim text-[9px] text-center">
            Click to expand {agentVotes.length} agent votes
          </div>
        )}
      </div>
    </div>
  );
}
