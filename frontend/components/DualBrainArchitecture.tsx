"use client";

import { useState, useEffect } from "react";

interface TABrainOutput {
  radar_score: { score: number; tier: string };
  pulse_signal: { tier: number; direction: string; strength: string };
  regime: string;
  strategy_signals: Array<{
    strategy: string;
    signal: string;
    confidence: string;
    reason: string;
  }>;
  overall_sentiment: string;
  overall_confidence: string;
}

interface FundBrainOutput {
  whale_sentiment: string;
  twitter_sentiment: string;
  twitter_data?: {
    mention_count: number;
    kol_mentions: Array<{ handle: string; sentiment: string; text: string }>;
    trending_score: number;
  };
  news_sentiment: string;
  security?: {
    is_honeypot?: boolean;
    risk_score?: number;
  };
  overall_sentiment: string;
}

interface MergedContext {
  symbol: string;
  ta: TABrainOutput;
  fund: FundBrainOutput;
  merged_sentiment: string;
  merged_confidence: string;
  consensus_action: string;
  is_conflict: boolean;
  size_factor: string;
  regime: string;
}

interface DualBrainData {
  timestamp: string;
  mode: string;
  best_opportunity: string;
  contexts: Record<string, MergedContext>;
}

export function DualBrainArchitecture() {
  const [data, setData] = useState<DualBrainData | null>(null);
  const [selectedSymbol, setSelectedSymbol] = useState<string>("");

  useEffect(() => {
    const fetchData = async () => {
      try {
        const res = await fetch("/api/brain/dual-context");
        if (res.ok) {
          const json = await res.json();
          setData(json);
          if (!selectedSymbol && json.best_opportunity) {
            setSelectedSymbol(json.best_opportunity);
          }
        }
      } catch (e) {
        console.error("Failed to fetch dual brain context:", e);
      }
    };
    fetchData();
    const interval = setInterval(fetchData, 5000);
    return () => clearInterval(interval);
  }, [selectedSymbol]);

  const ctx = data?.contexts?.[selectedSymbol];
  const symbols = Object.keys(data?.contexts || {});

  return (
    <div className="grid-cell h-full">
      <div className="grid-cell-header">
        <span className="grid-cell-title">SIMMONS DUAL BRAIN v3.0</span>
        <div className="flex items-center gap-3">
          <select
            value={selectedSymbol}
            onChange={(e) => setSelectedSymbol(e.target.value)}
            className="bg-bb-black border border-bb-border px-2 py-1 text-[9px] text-bb-amber"
          >
            {symbols.map((s) => (
              <option key={s} value={s}>{s}</option>
            ))}
          </select>
          <span className={`text-[9px] uppercase font-bold ${data?.mode === "live" ? "text-bb-green" : "text-bb-amber"}`}>
            {data?.mode || "loading"}
          </span>
        </div>
      </div>

      <div className="grid-cell-body overflow-auto p-3">
        {/* Two-column brain layout - always side by side */}
        <div className="grid grid-cols-2 gap-3 mb-3">
          {/* TA BRAIN */}
          <TABrainPanel ta={ctx?.ta} regime={ctx?.regime} />

          {/* FUNDAMENTAL BRAIN */}
          <FundBrainPanel fund={ctx?.fund} />
        </div>

        {/* Lower panels - 3 column horizontal */}
        <div className="grid grid-cols-3 gap-3">
          {/* Consensus Layer */}
          <ConsensusPanel ctx={ctx} />

          {/* Claude Orchestrator */}
          <OrchestratorPanel ctx={ctx} />

          {/* Execution Layer */}
          <ExecutionPanel mode={data?.mode} />
        </div>
      </div>
    </div>
  );
}

function TABrainPanel({ ta, regime }: { ta?: TABrainOutput; regime?: string }) {
  const radarScore = ta?.radar_score?.score ?? 0;
  const radarTier = ta?.radar_score?.tier ?? "loading";
  const pulseTier = ta?.pulse_signal?.tier ?? 0;
  const pulseDir = ta?.pulse_signal?.direction ?? "neutral";
  const strategies = ta?.strategy_signals ?? [];

  const getRadarColor = (score: number) => {
    if (score >= 250) return "text-bb-green";
    if (score >= 170) return "text-bb-amber";
    if (score >= 140) return "text-bb-orange";
    return "text-bb-red";
  };

  return (
    <div className="border border-bb-border bg-bb-panel p-3 h-full">
      <div className="text-[11px] font-bold text-bb-orange mb-3 tracking-wider">
        TA BRAIN <span className="text-bb-dim">(Nunchi 14 Strats)</span>
      </div>

      {/* APEX Orchestrator */}
      <div className="border border-bb-border bg-bb-surface p-2.5 mb-3">
        <div className="text-[9px] text-bb-cyan font-bold mb-1.5">APEX ORCHESTRATOR</div>
        <div className="grid grid-cols-3 gap-2 text-[8px] text-bb-dim">
          <div>2-3 concurrent slots</div>
          <div>Entry priority tiers</div>
          <div>ROE-based exits</div>
        </div>
      </div>

      {/* RADAR / PULSE / GUARD */}
      <div className="grid grid-cols-3 gap-2 mb-3">
        <div className="border border-bb-border bg-bb-black p-2.5 text-center">
          <div className="text-[8px] text-bb-dim mb-1">RADAR (15m)</div>
          <div className={`text-[18px] font-bold ${getRadarColor(radarScore)}`}>{radarScore}</div>
          <div className="text-[8px] text-bb-dim mt-1">{radarTier}</div>
        </div>
        <div className="border border-bb-border bg-bb-black p-2.5 text-center">
          <div className="text-[8px] text-bb-dim mb-1">PULSE (60s)</div>
          <div className="text-[18px] font-bold text-bb-amber">T{pulseTier}</div>
          <div className="text-[8px] text-bb-dim mt-1">{pulseDir}</div>
        </div>
        <div className="border border-bb-border bg-bb-black p-2.5 text-center">
          <div className="text-[8px] text-bb-dim mb-1">GUARD (tick)</div>
          <div className="text-[18px] font-bold text-bb-cyan">2&#966;</div>
          <div className="text-[8px] text-bb-dim mt-1">stops</div>
        </div>
      </div>

      {/* 14 Strategies + Signals side by side */}
      <div className="grid grid-cols-2 gap-2 mb-3">
        <div className="border border-bb-border bg-bb-surface p-2.5">
          <div className="text-[9px] text-bb-cyan font-bold mb-1.5">14 STRATEGIES</div>
          <div className="text-[8px] text-bb-dim space-y-1">
            <div><span className="text-bb-amber font-bold">MM:</span> engine, avellaneda, regime, grid, liq</div>
            <div><span className="text-bb-amber font-bold">ARB:</span> funding, basis</div>
            <div><span className="text-bb-amber font-bold">DIR:</span> momentum, mean_rev</div>
            <div><span className="text-bb-amber font-bold">INF:</span> hedge, rfq, claude</div>
          </div>
        </div>
        <div className="border border-bb-border bg-bb-surface p-2.5">
          <div className="text-[9px] text-bb-cyan font-bold mb-1.5">LIVE SIGNALS</div>
          <div className="space-y-1.5">
            {strategies.slice(0, 5).map((s, i) => (
              <div key={i} className="flex items-center justify-between text-[8px] border-b border-bb-border/50 pb-1">
                <span className="text-bb-dim">{s.strategy}</span>
                <span className={s.signal === "buy" ? "text-bb-green font-bold" : s.signal === "sell" ? "text-bb-red font-bold" : "text-bb-amber"}>
                  {s.signal.toUpperCase()}
                </span>
                <span className="text-bb-dim">{(parseFloat(s.confidence) * 100).toFixed(0)}%</span>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Output */}
      <div className="pt-2 border-t border-bb-border">
        <div className="text-[8px] text-bb-dim">Output: TABrainOutput</div>
        <div className="text-[8px] text-bb-muted">
          radar_score ({radarScore}) &bull; pulse_tier ({pulseTier}) &bull; regime ({regime})
        </div>
      </div>
    </div>
  );
}

function FundBrainPanel({ fund }: { fund?: FundBrainOutput }) {
  const whaleSent = parseFloat(fund?.whale_sentiment ?? "0");
  const twitterSent = parseFloat(fund?.twitter_sentiment ?? "0");
  const newsSent = parseFloat(fund?.news_sentiment ?? "0");
  const twitterData = fund?.twitter_data;

  const getSentimentColor = (val: number) => {
    if (val > 0.3) return "text-bb-green";
    if (val < -0.3) return "text-bb-red";
    return "text-bb-amber";
  };

  return (
    <div className="border border-bb-border bg-bb-panel p-3 h-full">
      <div className="text-[11px] font-bold text-bb-orange mb-3 tracking-wider">
        FUNDAMENTAL BRAIN <span className="text-bb-dim">(Multi-Src)</span>
      </div>

      {/* Data sources - 2x2 grid */}
      <div className="grid grid-cols-2 gap-2 mb-3">
        {/* Whale Tracker */}
        <div className="border border-bb-border bg-bb-surface p-2.5">
          <div className="text-[9px] text-bb-cyan font-bold mb-1.5">WHALE TRACKER</div>
          <div className="text-[8px] text-bb-dim mb-1">OnchainOS smart money</div>
          <div className="flex justify-between items-center">
            <span className="text-[8px] text-bb-dim">Sentiment</span>
            <span className={`text-[12px] font-bold ${getSentimentColor(whaleSent)}`}>
              {(whaleSent * 100).toFixed(0)}%
            </span>
          </div>
        </div>

        {/* Twitter/X Sentiment */}
        <div className="border border-bb-border bg-bb-surface p-2.5">
          <div className="text-[9px] text-bb-cyan font-bold mb-1.5">TWITTER/X</div>
          <div className="text-[8px] text-bb-dim mb-1">KOL mentions{twitterData ? ` (${twitterData.mention_count})` : ""}</div>
          <div className="flex justify-between items-center">
            <span className="text-[8px] text-bb-dim">Sentiment</span>
            <span className={`text-[12px] font-bold ${getSentimentColor(twitterSent)}`}>
              {(twitterSent * 100).toFixed(0)}%
            </span>
          </div>
        </div>

        {/* News RSS Feeds */}
        <div className="border border-bb-border bg-bb-surface p-2.5">
          <div className="text-[9px] text-bb-cyan font-bold mb-1.5">NEWS RSS</div>
          <div className="text-[8px] text-bb-dim mb-1">CoinDesk, The Block, Decrypt</div>
          <div className="flex justify-between items-center">
            <span className="text-[8px] text-bb-dim">Sentiment</span>
            <span className={`text-[12px] font-bold ${getSentimentColor(newsSent)}`}>
              {(newsSent * 100).toFixed(0)}%
            </span>
          </div>
        </div>

        {/* Security Scanner */}
        <div className="border border-bb-border bg-bb-surface p-2.5">
          <div className="text-[9px] text-bb-cyan font-bold mb-1.5">SECURITY SCAN</div>
          <div className="text-[8px] text-bb-dim space-y-1">
            <div className="flex justify-between">
              <span>Honeypot</span>
              <span className={fund?.security?.is_honeypot ? "text-bb-red font-bold" : "text-bb-green font-bold"}>
                {fund?.security?.is_honeypot ? "RISK" : "SAFE"}
              </span>
            </div>
            <div className="flex justify-between">
              <span>Risk score</span>
              <span className={(fund?.security?.risk_score ?? 0) > 50 ? "text-bb-red font-bold" : "text-bb-green font-bold"}>
                {fund?.security?.risk_score ?? 0}/100
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* KOL Feed */}
      {twitterData && twitterData.kol_mentions.length > 0 && (
        <div className="border border-bb-border bg-bb-surface p-2.5 mb-3">
          <div className="text-[9px] text-bb-cyan font-bold mb-1.5">KOL FEED</div>
          <div className="space-y-1">
            {twitterData.kol_mentions.slice(0, 3).map((kol, i) => (
              <div key={i} className="flex items-center gap-2 text-[8px]">
                <span className="text-bb-amber font-bold">{kol.handle}</span>
                <span className={kol.sentiment === "bullish" ? "text-bb-green" : kol.sentiment === "bearish" ? "text-bb-red" : "text-bb-dim"}>
                  {kol.sentiment}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Output */}
      <div className="pt-2 border-t border-bb-border">
        <div className="text-[8px] text-bb-dim">Output: FundBrainOutput</div>
        <div className="text-[8px] text-bb-muted">
          whale ({(whaleSent * 100).toFixed(0)}%) &bull; twitter ({(twitterSent * 100).toFixed(0)}%) &bull; news ({(newsSent * 100).toFixed(0)}%)
        </div>
      </div>
    </div>
  );
}

function ConsensusPanel({ ctx }: { ctx?: MergedContext }) {
  const mergedSent = parseFloat(ctx?.merged_sentiment ?? "0");
  const mergedConf = parseFloat(ctx?.merged_confidence ?? "0");
  const isConflict = ctx?.is_conflict ?? false;

  return (
    <div className="border border-bb-border bg-bb-surface p-3 h-full">
      <div className="flex items-center justify-between mb-3">
        <div className="text-[10px] font-bold text-bb-cyan tracking-wider">CONSENSUS LAYER</div>
        <span className={`text-[8px] px-2 py-0.5 ${isConflict ? "bg-bb-red/20 text-bb-red border border-bb-red/50" : "bg-bb-green/20 text-bb-green border border-bb-green/50"}`}>
          {isConflict ? "CONFLICT" : "ALIGNED"}
        </span>
      </div>

      <div className="space-y-2 text-[9px]">
        <div className="flex justify-between text-bb-dim">
          <span className="text-bb-muted">Merge</span>
          <span><span className="text-bb-amber">TA 60%</span> + <span className="text-bb-cyan">Fund 40%</span></span>
        </div>
        <div className="flex justify-between text-bb-dim">
          <span className="text-bb-muted">Sentiment</span>
          <span className={mergedSent > 0 ? "text-bb-green font-bold" : "text-bb-red font-bold"}>
            {(mergedSent * 100).toFixed(0)}%
          </span>
        </div>
        <div className="flex justify-between text-bb-dim">
          <span className="text-bb-muted">Confidence</span>
          <span className="text-bb-bright font-bold">{(mergedConf * 100).toFixed(0)}%</span>
        </div>
        <div className="flex justify-between text-bb-dim">
          <span className="text-bb-muted">Action</span>
          <span className={
            ctx?.consensus_action === "long" ? "text-bb-green font-bold" :
            ctx?.consensus_action === "short" ? "text-bb-red font-bold" :
            "text-bb-amber font-bold"
          }>
            {ctx?.consensus_action?.toUpperCase() ?? "HOLD"}
          </span>
        </div>
      </div>

      <div className="text-[7px] text-bb-muted mt-3 space-y-0.5">
        <div>RADAR &gt; 170 + Positive Fund = Entry</div>
        <div>Conflict triggers Claude debate</div>
        <div>Adaptive weights via REFLECT</div>
      </div>
    </div>
  );
}

function OrchestratorPanel({ ctx }: { ctx?: MergedContext }) {
  return (
    <div className="border border-bb-border bg-bb-panel p-3 h-full">
      <div className="text-[10px] font-bold text-bb-orange tracking-wider mb-3">
        CLAUDE ORCHESTRATOR <span className="text-bb-dim">(/simmons-dual)</span>
      </div>

      <div className="space-y-2">
        <div className="border border-bb-border bg-bb-black p-2">
          <div className="text-[8px] text-bb-cyan mb-0.5">Multi-Agent Debate</div>
          <div className="text-[9px] text-bb-dim">Bull / Bear / Risk</div>
        </div>
        <div className="border border-bb-border bg-bb-black p-2">
          <div className="text-[8px] text-bb-cyan mb-0.5">Strategy Select</div>
          <div className="text-[9px] text-bb-dim">MM vs ARB vs DIR</div>
        </div>
        <div className="border border-bb-border bg-bb-black p-2">
          <div className="text-[8px] text-bb-cyan mb-0.5">Position Sizing</div>
          <div className="text-[9px] text-bb-dim">Kelly: {((parseFloat(ctx?.size_factor ?? "0") * 100)).toFixed(0)}%</div>
        </div>
        <div className="border border-bb-border bg-bb-black p-2">
          <div className="text-[8px] text-bb-cyan mb-0.5">GUARD Stops</div>
          <div className="text-[9px] text-bb-dim">Synced</div>
        </div>
      </div>
    </div>
  );
}

function ExecutionPanel({ mode }: { mode?: string }) {
  return (
    <div className="border border-bb-border bg-bb-surface p-3 h-full">
      <div className="text-[10px] font-bold text-bb-cyan tracking-wider mb-3">EXECUTION LAYER</div>

      <div className="space-y-2">
        <div className={`border p-2 ${mode === "paper" ? "border-bb-amber bg-bb-amber/10" : "border-bb-border bg-bb-black"}`}>
          <div className="text-[8px] text-bb-amber mb-0.5">Paper</div>
          <div className="text-[9px] text-bb-dim">Simmons Rust engine</div>
        </div>
        <div className={`border p-2 ${mode === "live" ? "border-bb-green bg-bb-green/10" : "border-bb-border bg-bb-black"}`}>
          <div className="text-[8px] text-bb-green mb-0.5">Live DEX</div>
          <div className="text-[9px] text-bb-dim">OnchainOS swap</div>
        </div>
        <div className="border border-bb-border bg-bb-black p-2">
          <div className="text-[8px] text-bb-blue mb-0.5">Live Perps</div>
          <div className="text-[9px] text-bb-dim">Hyperliquid (opt)</div>
        </div>
      </div>

      <div className="text-[8px] text-bb-muted mt-3">
        Solana &bull; Base &bull; ETH &bull; Arbitrum
      </div>
    </div>
  );
}
