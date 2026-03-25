"use client";

import { memo } from "react";
import type { FundBrainData } from "@/lib/types";

interface FundBrainProps {
  fundBrain: FundBrainData | null;
}

const PLACEHOLDER_HEADLINES = [
  { source: "COINDESK", text: "BTC BREAKS $68K ON INSTITUTIONAL INFLOWS", sentiment: "positive" as const, time: "14:32" },
  { source: "THEBLOCK", text: "X LAYER TVL SURGES PAST $1.2B MILESTONE", sentiment: "positive" as const, time: "14:21" },
  { source: "DECRYPT", text: "SEC APPROVES SPOT ETH ETF OPTIONS", sentiment: "positive" as const, time: "14:09" },
  { source: "COINDESK", text: "WHALE MOVES 2400 ETH TO BINANCE", sentiment: "negative" as const, time: "13:48" },
  { source: "THEBLOCK", text: "PBOC CUTS RRR — RISK APPETITE RISES", sentiment: "positive" as const, time: "13:30" },
];

const PLACEHOLDER_WHALE_MOVES = [
  "0x7a2...f3d bought 142 ETH @ $3,842",
  "0x3b1...c8e sold 500K USDC for SOL",
  "0xe4f...a12 deposited $2.1M to Aave",
];

function sentimentBar(value: number) {
  const pct = ((value + 1) / 2) * 100;
  const color = value > 0.2 ? "bg-bb-green" : value < -0.2 ? "bg-bb-red" : "bg-bb-amber";
  return { pct, color };
}

function sentimentLabel(value: number) {
  if (value > 0.5) return { text: "BULLISH", color: "text-bb-green" };
  if (value > 0.2) return { text: "POSITIVE", color: "text-bb-green" };
  if (value > -0.2) return { text: "NEUTRAL", color: "text-bb-amber" };
  if (value > -0.5) return { text: "NEGATIVE", color: "text-bb-red" };
  return { text: "BEARISH", color: "text-bb-red" };
}

const sColor: Record<string, string> = { positive: "text-bb-green", negative: "text-bb-red", neutral: "text-bb-dim" };
const sIcon: Record<string, string> = { positive: "▲", negative: "▼", neutral: "●" };

export const FundBrain = memo(function FundBrain({ fundBrain }: FundBrainProps) {
  const whale = fundBrain?.whale_sentiment ?? 0.45;
  const twitter = fundBrain?.twitter_sentiment ?? 0.32;
  const news = fundBrain?.news_sentiment ?? 0.18;
  const flags = fundBrain?.security_flags ?? [];
  const whaleMoves = fundBrain?.whale_tracker?.recent_moves ?? PLACEHOLDER_WHALE_MOVES;
  const headlines = fundBrain?.news_headlines ?? PLACEHOLDER_HEADLINES;
  const trending = fundBrain?.trending_tokens ?? ["SOL", "ETH", "ONDO", "JUP"];

  const sources = [
    { key: "WHALE", icon: "🐋", value: whale },
    { key: "TWITTER", icon: "𝕏", value: twitter },
    { key: "NEWS", icon: "📰", value: news },
  ];

  return (
    <div className="bg-bb-surface border border-bb-border h-full flex flex-col overflow-hidden">
      <div className="px-2 py-1 border-b border-bb-border flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="text-bb-magenta font-bold text-[10px]">FUND BRAIN</span>
          <span className="text-bb-dim text-[9px]">MULTI-SRC</span>
        </div>
        <div className="flex items-center gap-1">
          {flags.length > 0 ? (
            <span className="text-bb-red text-[9px] font-bold blink">{flags.length} FLAG{flags.length > 1 ? "S" : ""}</span>
          ) : (
            <span className="text-bb-green text-[9px]">● CLEAR</span>
          )}
        </div>
      </div>

      <div className="flex-1 min-h-0 overflow-y-auto">
        {/* Sentiment gauges */}
        <div className="border-b border-bb-border">
          {sources.map((src) => {
            const bar = sentimentBar(src.value);
            const label = sentimentLabel(src.value);
            return (
              <div key={src.key} className="px-2 py-1 border-b border-bb-border last:border-b-0">
                <div className="flex items-center justify-between text-[9px] mb-0.5">
                  <div className="flex items-center gap-1">
                    <span className="w-[14px] text-center">{src.icon}</span>
                    <span className="text-bb-dim">{src.key}</span>
                  </div>
                  <div className="flex items-center gap-1.5">
                    <span className="text-bb-bright font-bold">{src.value > 0 ? "+" : ""}{src.value.toFixed(2)}</span>
                    <span className={`font-bold ${label.color}`}>{label.text}</span>
                  </div>
                </div>
                <div className="h-1 bg-bb-raised relative">
                  {/* Center marker */}
                  <div className="absolute left-1/2 top-0 h-full w-px bg-bb-border-light" />
                  <div className={`h-full ${bar.color} transition-all`} style={{ width: `${bar.pct}%` }} />
                </div>
              </div>
            );
          })}
        </div>

        {/* Security Scanner */}
        <div className="px-2 py-1 border-b border-bb-border flex items-center justify-between text-[9px]">
          <div className="flex items-center gap-1">
            <span className="text-bb-dim">SECURITY</span>
          </div>
          {flags.length > 0 ? (
            <span className="text-bb-red font-bold">{flags.join(", ")}</span>
          ) : (
            <span className="text-bb-green font-bold">NO THREATS</span>
          )}
        </div>

        {/* Trending */}
        <div className="px-2 py-1 border-b border-bb-border flex items-center gap-1.5 text-[9px]">
          <span className="text-bb-dim shrink-0">TRENDING</span>
          {trending.map((t) => (
            <span key={t} className="text-bb-cyan font-bold">${t}</span>
          ))}
        </div>

        {/* Whale Moves */}
        <div className="border-b border-bb-border">
          <div className="px-2 py-0.5 bg-bb-raised text-[9px] text-bb-blue font-bold">WHALE TRACKER</div>
          {whaleMoves.map((move, i) => (
            <div key={i} className="px-2 py-0.5 text-[9px] border-b border-bb-border last:border-b-0">
              <span className="text-bb-cyan mr-1">&gt;</span>
              <span className="text-bb-white">{move}</span>
            </div>
          ))}
        </div>

        {/* News Headlines */}
        <div>
          <div className="px-2 py-0.5 bg-bb-raised text-[9px] text-bb-amber font-bold flex items-center justify-between">
            <span>NEWS RSS</span>
            <span className="text-bb-orange blink">● LIVE</span>
          </div>
          {headlines.map((h, i) => (
            <div key={i} className="px-2 py-0.5 text-[9px] border-b border-bb-border last:border-b-0 hover:bg-bb-raised">
              <div className="flex items-start gap-1 min-w-0">
                <span className="text-bb-dim shrink-0">{h.time}</span>
                <span className={`shrink-0 ${sColor[h.sentiment]}`}>{sIcon[h.sentiment]}</span>
                <span className="text-bb-cyan shrink-0 font-bold">{h.source}</span>
                <span className="text-bb-white truncate">{h.text}</span>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
});
