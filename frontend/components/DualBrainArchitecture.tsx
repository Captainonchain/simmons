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

      <div className="grid-cell-body overflow-auto p-2">
        {/* Two-column brain layout */}
        <div className="grid grid-cols-2 gap-2 mb-2">
          {/* TA BRAIN */}
          <TABrainPanel ta={ctx?.ta} regime={ctx?.regime} />

          {/* FUNDAMENTAL BRAIN */}
          <FundBrainPanel fund={ctx?.fund} />
        </div>

        {/* Consensus Layer */}
        <ConsensusPanel ctx={ctx} />

        {/* Claude Orchestrator */}
        <OrchestratorPanel ctx={ctx} />

        {/* Execution Layer */}
        <ExecutionPanel mode={data?.mode} />
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
    <div className="border border-bb-border bg-bb-panel p-2">
      <div className="text-[10px] font-bold text-bb-orange mb-2 tracking-wider">
        TA BRAIN <span className="text-bb-dim">(Nunchi 14 Strats)</span>
      </div>

      {/* APEX Orchestrator */}
      <div className="border border-bb-border bg-bb-surface p-2 mb-2">
        <div className="text-[8px] text-bb-cyan font-bold mb-1">APEX ORCHESTRATOR</div>
        <div className="text-[8px] text-bb-dim space-y-0.5">
          <div>• 2-3 concurrent slots</div>
          <div>• Entry priority tiers</div>
          <div>• ROE-based exits</div>
        </div>
      </div>

      {/* RADAR / PULSE / GUARD */}
      <div className="grid grid-cols-3 gap-1 mb-2">
        <div className="border border-bb-border bg-bb-black p-1.5 text-center">
          <div className="text-[7px] text-bb-dim">RADAR (15m)</div>
          <div className={`text-[14px] font-bold ${getRadarColor(radarScore)}`}>{radarScore}</div>
          <div className="text-[7px] text-bb-dim">{radarTier}</div>
        </div>
        <div className="border border-bb-border bg-bb-black p-1.5 text-center">
          <div className="text-[7px] text-bb-dim">PULSE (60s)</div>
          <div className="text-[14px] font-bold text-bb-amber">T{pulseTier}</div>
          <div className="text-[7px] text-bb-dim">{pulseDir}</div>
        </div>
        <div className="border border-bb-border bg-bb-black p-1.5 text-center">
          <div className="text-[7px] text-bb-dim">GUARD (tick)</div>
          <div className="text-[14px] font-bold text-bb-cyan">2φ</div>
          <div className="text-[7px] text-bb-dim">stops</div>
        </div>
      </div>

      {/* 14 Strategies */}
      <div className="border border-bb-border bg-bb-surface p-2 mb-2">
        <div className="text-[8px] text-bb-cyan font-bold mb-1">14 STRATEGIES:</div>
        <div className="text-[7px] text-bb-dim space-y-0.5">
          <div><span className="text-bb-amber">MM:</span> engine, avellaneda, regime, grid, liq</div>
          <div><span className="text-bb-amber">ARB:</span> funding, basis</div>
          <div><span className="text-bb-amber">DIR:</span> momentum, mean_rev</div>
          <div><span className="text-bb-amber">INF:</span> hedge, rfq, claude</div>
        </div>
      </div>

      {/* Strategy Signals */}
      <div className="space-y-1">
        {strategies.slice(0, 4).map((s, i) => (
          <div key={i} className="flex items-center justify-between text-[8px] border-b border-bb-border pb-1">
            <span className="text-bb-dim">{s.strategy}</span>
            <span className={s.signal === "buy" ? "text-bb-green" : s.signal === "sell" ? "text-bb-red" : "text-bb-amber"}>
              {s.signal.toUpperCase()}
            </span>
            <span className="text-bb-dim">{(parseFloat(s.confidence) * 100).toFixed(0)}%</span>
          </div>
        ))}
      </div>

      {/* Output */}
      <div className="mt-2 pt-2 border-t border-bb-border">
        <div className="text-[7px] text-bb-dim">Output: TABrainOutput</div>
        <div className="text-[7px] text-bb-muted">
          • radar_score ({radarScore}) • pulse_tier ({pulseTier}) • regime ({regime})
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
    <div className="border border-bb-border bg-bb-panel p-2">
      <div className="text-[10px] font-bold text-bb-orange mb-2 tracking-wider">
        FUNDAMENTAL BRAIN <span className="text-bb-dim">(Multi-Src)</span>
      </div>

      {/* Whale Tracker */}
      <div className="border border-bb-border bg-bb-surface p-2 mb-2">
        <div className="text-[8px] text-bb-cyan font-bold mb-1">WHALE TRACKER (OnchainOS)</div>
        <div className="flex justify-between items-center">
          <span className="text-[8px] text-bb-dim">Smart money signals</span>
          <span className={`text-[10px] font-bold ${getSentimentColor(whaleSent)}`}>
            {(whaleSent * 100).toFixed(0)}%
          </span>
        </div>
        <div className="text-[7px] text-bb-dim">• KOL tracking</div>
      </div>

      {/* Twitter/X Sentiment */}
      <div className="border border-bb-border bg-bb-surface p-2 mb-2">
        <div className="text-[8px] text-bb-cyan font-bold mb-1">TWITTER/X SENTIMENT</div>
        <div className="flex justify-between items-center">
          <span className="text-[8px] text-bb-dim">KOL mentions</span>
          <span className={`text-[10px] font-bold ${getSentimentColor(twitterSent)}`}>
            {(twitterSent * 100).toFixed(0)}%
          </span>
        </div>
        {twitterData && (
          <div className="text-[7px] text-bb-dim">
            • {twitterData.mention_count} mentions • Score: {twitterData.trending_score}
          </div>
        )}
      </div>

      {/* News RSS Feeds */}
      <div className="border border-bb-border bg-bb-surface p-2 mb-2">
        <div className="text-[8px] text-bb-cyan font-bold mb-1">NEWS RSS FEEDS</div>
        <div className="flex justify-between items-center">
          <span className="text-[8px] text-bb-dim">Headlines sentiment</span>
          <span className={`text-[10px] font-bold ${getSentimentColor(newsSent)}`}>
            {(newsSent * 100).toFixed(0)}%
          </span>
        </div>
        <div className="text-[7px] text-bb-dim">• CoinDesk, The Block, Decrypt</div>
      </div>

      {/* Security Scanner */}
      <div className="border border-bb-border bg-bb-surface p-2 mb-2">
        <div className="text-[8px] text-bb-cyan font-bold mb-1">SECURITY SCANNER</div>
        <div className="text-[7px] text-bb-dim space-y-0.5">
          <div className="flex justify-between">
            <span>Honeypot detection</span>
            <span className={fund?.security?.is_honeypot ? "text-bb-red" : "text-bb-green"}>
              {fund?.security?.is_honeypot ? "⚠ RISK" : "✓ SAFE"}
            </span>
          </div>
          <div className="flex justify-between">
            <span>Risk score</span>
            <span className={(fund?.security?.risk_score ?? 0) > 50 ? "text-bb-red" : "text-bb-green"}>
              {fund?.security?.risk_score ?? 0}/100
            </span>
          </div>
        </div>
      </div>

      {/* Output */}
      <div className="mt-2 pt-2 border-t border-bb-border">
        <div className="text-[7px] text-bb-dim">Output: FundBrainOutput</div>
        <div className="text-[7px] text-bb-muted">
          • whale ({(whaleSent * 100).toFixed(0)}%) • twitter ({(twitterSent * 100).toFixed(0)}%) • news ({(newsSent * 100).toFixed(0)}%)
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
    <div className="border border-bb-border bg-bb-surface p-2 mb-2">
      <div className="flex items-center justify-between mb-2">
        <div className="text-[9px] font-bold text-bb-cyan tracking-wider">CONSENSUS LAYER</div>
        <div className="flex items-center gap-2">
          <span className={`text-[8px] px-1.5 py-0.5 ${isConflict ? "bg-bb-red/20 text-bb-red border border-bb-red/50" : "bg-bb-green/20 text-bb-green border border-bb-green/50"}`}>
            {isConflict ? "CONFLICT" : "ALIGNED"}
          </span>
        </div>
      </div>

      <div className="grid grid-cols-4 gap-2 text-[8px]">
        <div className="text-bb-dim">
          <div className="text-[7px] text-bb-muted mb-0.5">Merge</div>
          <span className="text-bb-amber">TA (60%)</span> + <span className="text-bb-cyan">Fund (40%)</span>
        </div>
        <div className="text-bb-dim">
          <div className="text-[7px] text-bb-muted mb-0.5">Sentiment</div>
          <span className={mergedSent > 0 ? "text-bb-green" : "text-bb-red"}>
            {(mergedSent * 100).toFixed(0)}%
          </span>
        </div>
        <div className="text-bb-dim">
          <div className="text-[7px] text-bb-muted mb-0.5">Confidence</div>
          <span className="text-bb-bright">{(mergedConf * 100).toFixed(0)}%</span>
        </div>
        <div className="text-bb-dim">
          <div className="text-[7px] text-bb-muted mb-0.5">Action</div>
          <span className={
            ctx?.consensus_action === "long" ? "text-bb-green font-bold" :
            ctx?.consensus_action === "short" ? "text-bb-red font-bold" :
            "text-bb-amber"
          }>
            {ctx?.consensus_action?.toUpperCase() ?? "HOLD"}
          </span>
        </div>
      </div>

      <div className="text-[7px] text-bb-muted mt-2 flex gap-4">
        <span>• Entry: RADAR &gt; 170 + Positive Fund</span>
        <span>• Conflict: Trigger Claude debate</span>
        <span>• Adaptive weights via REFLECT</span>
      </div>
    </div>
  );
}

function OrchestratorPanel({ ctx }: { ctx?: MergedContext }) {
  return (
    <div className="border border-bb-border bg-bb-panel p-2 mb-2">
      <div className="text-[9px] font-bold text-bb-orange tracking-wider mb-2">
        CLAUDE ORCHESTRATOR <span className="text-bb-dim">(/simmons-dual)</span>
      </div>

      <div className="grid grid-cols-4 gap-2 text-[8px] text-bb-dim">
        <div className="border border-bb-border bg-bb-black p-1.5">
          <div className="text-[7px] text-bb-cyan mb-0.5">Debate</div>
          Bull/Bear/Risk
        </div>
        <div className="border border-bb-border bg-bb-black p-1.5">
          <div className="text-[7px] text-bb-cyan mb-0.5">Strategy</div>
          MM vs ARB vs DIR
        </div>
        <div className="border border-bb-border bg-bb-black p-1.5">
          <div className="text-[7px] text-bb-cyan mb-0.5">Size</div>
          Kelly: {((parseFloat(ctx?.size_factor ?? "0") * 100)).toFixed(0)}%
        </div>
        <div className="border border-bb-border bg-bb-black p-1.5">
          <div className="text-[7px] text-bb-cyan mb-0.5">GUARD</div>
          Stops synced
        </div>
      </div>
    </div>
  );
}

function ExecutionPanel({ mode }: { mode?: string }) {
  return (
    <div className="border border-bb-border bg-bb-surface p-2">
      <div className="text-[9px] font-bold text-bb-cyan tracking-wider mb-2">EXECUTION LAYER</div>

      <div className="grid grid-cols-3 gap-2 text-[8px]">
        <div className={`border p-1.5 ${mode === "paper" ? "border-bb-amber bg-bb-amber/10" : "border-bb-border bg-bb-black"}`}>
          <div className="text-[7px] text-bb-amber mb-0.5">Paper</div>
          <span className="text-bb-dim">Simmons Rust engine</span>
        </div>
        <div className={`border p-1.5 ${mode === "live" ? "border-bb-green bg-bb-green/10" : "border-bb-border bg-bb-black"}`}>
          <div className="text-[7px] text-bb-green mb-0.5">Live DEX</div>
          <span className="text-bb-dim">OnchainOS swap</span>
        </div>
        <div className="border border-bb-border bg-bb-black p-1.5">
          <div className="text-[7px] text-bb-blue mb-0.5">Live Perps</div>
          <span className="text-bb-dim">Hyperliquid (opt)</span>
        </div>
      </div>

      <div className="text-[7px] text-bb-muted mt-2">
        Chains: Solana • Base • ETH • Arbitrum
      </div>
    </div>
  );
}
