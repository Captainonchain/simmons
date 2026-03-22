"use client";

import { memo, useEffect, useState } from "react";

interface Headline {
  time: string;
  source: string;
  text: string;
  sentiment: "positive" | "negative" | "neutral";
}

const PLACEHOLDER_HEADLINES: Headline[] = [
  { time: "14:32", source: "OKXNEWS", text: "BTC BREAKS ABOVE $68K RESISTANCE LEVEL ON HIGH VOLUME", sentiment: "positive" },
  { time: "14:28", source: "ONCHAIN", text: "WHALE WALLET MOVES 2,400 ETH TO BINANCE — POSSIBLE SELL PRESSURE", sentiment: "negative" },
  { time: "14:21", source: "XLAYER", text: "X LAYER TVL REACHES $1.2B MILESTONE AFTER DEX SURGE", sentiment: "positive" },
  { time: "14:15", source: "DEFI", text: "AAVE V3 PROPOSAL TO ADD NEW COLLATERAL TYPES PASSES GOVERNANCE", sentiment: "neutral" },
  { time: "14:09", source: "FED", text: "FED MINUTES SIGNAL POTENTIAL RATE HOLD THROUGH Q3 2026", sentiment: "positive" },
  { time: "14:04", source: "ECON", text: "US CPI COMES IN AT 2.8% YOY — BELOW CONSENSUS 2.9% ESTIMATE", sentiment: "positive" },
  { time: "13:58", source: "GEOPO", text: "EU PASSES LANDMARK DIGITAL ASSET REGULATION FRAMEWORK — MICA II", sentiment: "neutral" },
  { time: "13:52", source: "RATES", text: "10Y TREASURY YIELD DROPS TO 4.12% — RISK-ON ROTATION UNDERWAY", sentiment: "positive" },
  { time: "13:48", source: "EQUIT", text: "S&P 500 HITS ALL-TIME HIGH — TECH LEADS WITH NVIDIA +4.2%", sentiment: "positive" },
  { time: "13:42", source: "FOREX", text: "DXY DROPS BELOW 103 — DOLLAR WEAKNESS FAVORS CRYPTO ASSETS", sentiment: "positive" },
  { time: "13:36", source: "CMDTY", text: "GOLD TOUCHES $2,450/OZ — SAFE HAVEN DEMAND PERSISTS ALONGSIDE BTC", sentiment: "neutral" },
  { time: "13:30", source: "CHINA", text: "PBOC CUTS RRR BY 50BPS — LIQUIDITY INJECTION BOOSTS RISK APPETITE", sentiment: "positive" },
  { time: "13:24", source: "NUNCHI", text: "SENTIMENT SHIFT DETECTED — FUNDING RATES TURNING NEGATIVE ON ALTS", sentiment: "negative" },
  { time: "13:18", source: "ONCHAIN", text: "STABLECOIN INFLOWS TO EXCHANGES UP 340% IN LAST 24H", sentiment: "positive" },
  { time: "13:12", source: "SOCIAL", text: "CRYPTO TWITTER VOLUME SPIKE ON $SOL — BREAKOUT PATTERN FORMING", sentiment: "neutral" },
  { time: "13:06", source: "REGUL", text: "SEC APPROVES SPOT ETH ETF OPTIONS TRADING — EFFECTIVE NEXT MONTH", sentiment: "positive" },
  { time: "13:00", source: "ENRGY", text: "BRENT CRUDE FALLS TO $72/BBL — OPEC+ SIGNALS OUTPUT INCREASE", sentiment: "negative" },
  { time: "12:54", source: "GEOPO", text: "US-CHINA TRADE TALKS RESUME IN GENEVA — TARIFF ROLLBACK ON TABLE", sentiment: "positive" },
  { time: "12:48", source: "LABOR", text: "US JOBLESS CLAIMS AT 215K — LABOR MARKET REMAINS TIGHT", sentiment: "neutral" },
  { time: "12:42", source: "CORP", text: "BLACKROCK FILES FOR TOKENIZED MONEY MARKET FUND ON ETHEREUM", sentiment: "positive" },
];

const sColor: Record<string, string> = { positive: "text-bb-green", negative: "text-bb-red", neutral: "text-bb-dim" };
const sIcon: Record<string, string> = { positive: "▲", negative: "▼", neutral: "●" };

export const Headlines = memo(function Headlines() {
  const [headlines] = useState<Headline[]>(PLACEHOLDER_HEADLINES);
  const [tick, setTick] = useState(0);

  useEffect(() => {
    const id = setInterval(() => setTick((t) => t + 1), 1000);
    return () => clearInterval(id);
  }, []);

  return (
    <div className="bg-bb-surface border border-bb-border h-full flex flex-col">
      <div className="px-2 py-1 border-b border-bb-border flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="text-bb-amber font-bold text-[10px]">HEADLINES</span>
        </div>
        <span className={`text-bb-orange text-[9px] ${tick % 2 === 0 ? "opacity-100" : "opacity-30"}`}>● LIVE</span>
      </div>
      <div className="overflow-y-auto flex-1 min-h-0">
        {headlines.map((h, i) => (
          <div key={i} className={`px-2 py-1 border-b border-bb-border last:border-b-0 hover:bg-bb-raised cursor-pointer text-[10px] ${i === 0 ? "bg-bb-raised" : ""}`}>
            <div className="flex items-start gap-1 sm:gap-1.5">
              <span className="text-bb-dim shrink-0 w-[30px] sm:w-[34px]">{h.time}</span>
              <span className={`shrink-0 ${sColor[h.sentiment]}`}>{sIcon[h.sentiment]}</span>
              <span className="text-bb-cyan shrink-0 w-[40px] sm:w-[52px] font-bold truncate">{h.source}</span>
              <span className="text-bb-white truncate sm:whitespace-normal">{h.text}</span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
});
