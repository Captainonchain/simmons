"use client";

import { memo, useState, useEffect } from "react";

interface AgentVote {
  agent: string;
  role: "analyst" | "researcher" | "risk";
  recommendation: "BUY" | "SELL" | "HOLD" | "LONG" | "SHORT" | "CAUTION" | string;
  confidence: number;
  reason: string;
}

interface AgentDebateProps {
  bullConviction?: number;
  bearConviction?: number;
  finalDecision?: "BUY" | "SELL" | "HOLD" | "WAIT" | null;
  agentVotes?: AgentVote[];
  isLive?: boolean;
}

const DEFAULT_VOTES: AgentVote[] = [
  { agent: "Technical", role: "analyst", recommendation: "BUY", confidence: 0.72, reason: "MACD crossover, RSI oversold" },
  { agent: "Fundamental", role: "analyst", recommendation: "HOLD", confidence: 0.58, reason: "Fair valuation, growth neutral" },
  { agent: "Sentiment", role: "analyst", recommendation: "BUY", confidence: 0.68, reason: "Smart money accumulating" },
  { agent: "On-chain", role: "analyst", recommendation: "BUY", confidence: 0.85, reason: "Whale accumulation detected" },
  { agent: "Bull", role: "researcher", recommendation: "LONG", confidence: 0.78, reason: "Multi-signal convergence" },
  { agent: "Bear", role: "researcher", recommendation: "CAUTION", confidence: 0.45, reason: "Resistance at $68K" },
  { agent: "Aggressive", role: "risk", recommendation: "15%", confidence: 0.72, reason: "Full Kelly position" },
  { agent: "Conservative", role: "risk", recommendation: "8%", confidence: 0.65, reason: "Half Kelly safer" },
  { agent: "Neutral", role: "risk", recommendation: "10%", confidence: 0.70, reason: "Balanced approach" },
];

export const AgentDebate = memo(function AgentDebate({
  bullConviction = 0.75,
  bearConviction = 0.45,
  finalDecision = "BUY",
  agentVotes = DEFAULT_VOTES,
  isLive = true,
}: AgentDebateProps) {
  const [activePhase, setActivePhase] = useState<"analysts" | "researchers" | "risk">("analysts");
  const [animatedBull, setAnimatedBull] = useState(0);
  const [animatedBear, setAnimatedBear] = useState(0);

  useEffect(() => {
    const timer = setTimeout(() => {
      setAnimatedBull(bullConviction);
      setAnimatedBear(bearConviction);
    }, 100);
    return () => clearTimeout(timer);
  }, [bullConviction, bearConviction]);

  const analysts = agentVotes.filter((v) => v.role === "analyst");
  const researchers = agentVotes.filter((v) => v.role === "researcher");
  const riskTeam = agentVotes.filter((v) => v.role === "risk");

  const getRecColor = (rec: string) => {
    const r = rec.toUpperCase();
    if (["BUY", "LONG"].includes(r)) return "text-bb-green bg-bb-green/15 border-bb-green/30";
    if (["SELL", "SHORT"].includes(r)) return "text-bb-red bg-bb-red/15 border-bb-red/30";
    if (["HOLD", "CAUTION"].includes(r)) return "text-bb-amber bg-bb-amber/15 border-bb-amber/30";
    return "text-bb-cyan bg-bb-cyan/15 border-bb-cyan/30";
  };

  const getDecisionStyle = (dec: string | null) => {
    if (!dec) return { color: "text-bb-dim", bg: "bg-bb-muted" };
    if (["BUY", "LONG"].includes(dec)) return { color: "text-bb-green", bg: "bg-bb-green/20" };
    if (["SELL", "SHORT"].includes(dec)) return { color: "text-bb-red", bg: "bg-bb-red/20" };
    return { color: "text-bb-amber", bg: "bg-bb-amber/20" };
  };

  const decStyle = getDecisionStyle(finalDecision);

  const renderVotes = (votes: AgentVote[]) => (
    <div className="grid grid-cols-2 gap-1.5">
      {votes.map((vote) => (
        <div key={vote.agent} className="bg-bb-raised border border-bb-border p-2 hover:border-bb-border-light transition-colors">
          <div className="flex items-center justify-between mb-1.5">
            <span className="text-bb-cyan text-[9px] font-semibold">{vote.agent}</span>
            <span className={`text-[8px] px-1.5 py-0.5 font-bold border ${getRecColor(vote.recommendation)}`}>
              {vote.recommendation}
            </span>
          </div>
          <div className="text-[8px] text-bb-dim leading-snug mb-2 line-clamp-2" title={vote.reason}>
            {vote.reason}
          </div>
          <div className="flex items-center gap-2">
            <div className="flex-1 h-1 bg-bb-border overflow-hidden">
              <div
                className="h-full bg-bb-cyan transition-all duration-500"
                style={{ width: `${vote.confidence * 100}%` }}
              />
            </div>
            <span className="text-[8px] text-bb-dim stat-value">{(vote.confidence * 100).toFixed(0)}%</span>
          </div>
        </div>
      ))}
    </div>
  );

  return (
    <div className="grid-cell h-full">
      <div className="grid-cell-header">
        <div className="flex items-center gap-2">
          <span className="grid-cell-title">AGENT DEBATE</span>
          {isLive && <span className="w-1.5 h-1.5 bg-bb-green rounded-full pulse" />}
        </div>
        <div className={`flex items-center gap-2 px-2 py-0.5 ${decStyle.bg}`}>
          <span className="text-[8px] text-bb-dim">DECISION</span>
          <span className={`text-[11px] font-bold ${decStyle.color}`}>
            {finalDecision || "PENDING"}
          </span>
        </div>
      </div>

      <div className="flex-1 flex flex-col min-h-0 overflow-hidden">
        {/* Conviction Bars */}
        <div className="px-3 py-2.5 border-b border-bb-border bg-bb-panel shrink-0">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <div className="flex items-center justify-between mb-1">
                <span className="text-[8px] text-bb-green font-medium">BULL</span>
                <span className="text-[10px] text-bb-green stat-value font-bold">
                  {(bullConviction * 100).toFixed(0)}%
                </span>
              </div>
              <div className="h-2 bg-bb-border overflow-hidden">
                <div
                  className="h-full bg-gradient-to-r from-bb-green/50 to-bb-green transition-all duration-700 ease-out"
                  style={{ width: `${animatedBull * 100}%` }}
                />
              </div>
            </div>
            <div>
              <div className="flex items-center justify-between mb-1">
                <span className="text-[8px] text-bb-red font-medium">BEAR</span>
                <span className="text-[10px] text-bb-red stat-value font-bold">
                  {(bearConviction * 100).toFixed(0)}%
                </span>
              </div>
              <div className="h-2 bg-bb-border overflow-hidden">
                <div
                  className="h-full bg-gradient-to-r from-bb-red/50 to-bb-red transition-all duration-700 ease-out"
                  style={{ width: `${animatedBear * 100}%` }}
                />
              </div>
            </div>
          </div>
        </div>

        {/* Phase Tabs */}
        <div className="flex border-b border-bb-border shrink-0">
          {[
            { id: "analysts", label: "ANALYSTS", count: analysts.length },
            { id: "researchers", label: "RESEARCH", count: researchers.length },
            { id: "risk", label: "RISK", count: riskTeam.length },
          ].map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActivePhase(tab.id as typeof activePhase)}
              className={`flex-1 py-2 text-[9px] font-medium transition-all relative ${
                activePhase === tab.id
                  ? "text-bb-orange bg-bb-raised"
                  : "text-bb-dim hover:text-bb-white hover:bg-bb-raised/50"
              }`}
            >
              {tab.label}
              <span className="ml-1 text-[8px] opacity-60">({tab.count})</span>
              {activePhase === tab.id && (
                <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-bb-orange" />
              )}
            </button>
          ))}
        </div>

        {/* Vote Grid */}
        <div className="flex-1 overflow-auto p-2">
          {activePhase === "analysts" && renderVotes(analysts)}
          {activePhase === "researchers" && renderVotes(researchers)}
          {activePhase === "risk" && renderVotes(riskTeam)}
        </div>
      </div>
    </div>
  );
});
