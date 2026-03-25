"use client";

interface CircuitBreakerProps {
  triggered?: boolean;
  reason?: string | null;
  riskLevel?: "normal" | "elevated" | "critical";
  currentDrawdown?: number;
  maxDrawdownLimit?: number;
  consecutiveLosses?: number;
  maxConsecutiveLosses?: number;
  positionSizeModifier?: number;
  canTrade?: boolean;
  recommendations?: string[];
}

export function CircuitBreaker({
  triggered = false,
  reason = null,
  riskLevel = "normal",
  currentDrawdown = 0,
  maxDrawdownLimit = 0.2,
  consecutiveLosses = 0,
  maxConsecutiveLosses = 3,
  positionSizeModifier = 1,
  canTrade = true,
  recommendations = [],
}: CircuitBreakerProps) {
  const getRiskColor = () => {
    switch (riskLevel) {
      case "critical": return "text-bb-red";
      case "elevated": return "text-bb-amber";
      default: return "text-bb-green";
    }
  };

  const getRiskBg = () => {
    switch (riskLevel) {
      case "critical": return "bg-bb-red/10 border-bb-red/30";
      case "elevated": return "bg-bb-amber/10 border-bb-amber/30";
      default: return "bg-bb-green/10 border-bb-green/30";
    }
  };

  const drawdownPct = currentDrawdown * 100;
  const drawdownLimitPct = maxDrawdownLimit * 100;
  const drawdownRatio = Math.min(1, currentDrawdown / maxDrawdownLimit);

  return (
    <div className="bg-bb-surface border border-bb-border h-full flex flex-col">
      <div className="bg-bb-panel px-2 py-1 border-b border-bb-border flex items-center justify-between">
        <span className="text-[10px] text-bb-orange font-semibold tracking-wider">CIRCUIT BREAKER</span>
        <div className="flex items-center gap-1.5">
          <span className={`w-2 h-2 rounded-full ${triggered ? "bg-bb-red blink" : "bg-bb-green"}`} />
          <span className={`text-[9px] ${triggered ? "text-bb-red" : "text-bb-green"}`}>
            {triggered ? "TRIGGERED" : "ACTIVE"}
          </span>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-2 space-y-2 text-[10px]">
        {/* Risk Level Badge */}
        <div className={`border p-2 ${getRiskBg()}`}>
          <div className="flex items-center justify-between">
            <span className="text-bb-dim">Risk Level</span>
            <span className={`font-bold uppercase ${getRiskColor()}`}>
              {riskLevel}
            </span>
          </div>
          <div className="flex items-center justify-between mt-1 text-[9px]">
            <span className="text-bb-dim">Position Modifier</span>
            <span className="text-bb-white">{(positionSizeModifier * 100).toFixed(0)}%</span>
          </div>
        </div>

        {/* Triggered Reason */}
        {triggered && reason && (
          <div className="bg-bb-red/10 border border-bb-red/50 p-2">
            <div className="text-bb-red font-bold text-[9px] uppercase mb-1">TRADING HALTED</div>
            <div className="text-bb-white">{reason}</div>
          </div>
        )}

        {/* Drawdown Meter */}
        <div className="bg-bb-raised border border-bb-border p-2">
          <div className="flex items-center justify-between mb-1">
            <span className="text-bb-dim">Drawdown</span>
            <span className={drawdownPct > 15 ? "text-bb-red" : drawdownPct > 10 ? "text-bb-amber" : "text-bb-green"}>
              {drawdownPct.toFixed(1)}% / {drawdownLimitPct}%
            </span>
          </div>
          <div className="h-2 bg-bb-border">
            <div
              className={`h-full transition-all ${
                drawdownPct > 15 ? "bg-bb-red" : drawdownPct > 10 ? "bg-bb-amber" : "bg-bb-green"
              }`}
              style={{ width: `${drawdownRatio * 100}%` }}
            />
          </div>
        </div>

        {/* Consecutive Losses */}
        <div className="bg-bb-raised border border-bb-border p-2">
          <div className="flex items-center justify-between mb-1">
            <span className="text-bb-dim">Consecutive Losses</span>
            <span className={consecutiveLosses >= 2 ? "text-bb-red" : "text-bb-white"}>
              {consecutiveLosses} / {maxConsecutiveLosses}
            </span>
          </div>
          <div className="flex gap-1">
            {Array.from({ length: maxConsecutiveLosses }).map((_, i) => (
              <div
                key={i}
                className={`flex-1 h-2 ${
                  i < consecutiveLosses ? "bg-bb-red" : "bg-bb-border"
                }`}
              />
            ))}
          </div>
        </div>

        {/* Trading Status */}
        <div className={`p-2 border ${canTrade ? "bg-bb-green/10 border-bb-green/30" : "bg-bb-red/10 border-bb-red/30"}`}>
          <div className="flex items-center gap-2">
            <span className={`text-lg ${canTrade ? "text-bb-green" : "text-bb-red"}`}>
              {canTrade ? "✓" : "✗"}
            </span>
            <div>
              <div className={`font-bold ${canTrade ? "text-bb-green" : "text-bb-red"}`}>
                {canTrade ? "TRADING ENABLED" : "TRADING BLOCKED"}
              </div>
              <div className="text-bb-dim text-[9px]">
                {canTrade
                  ? `Position size at ${(positionSizeModifier * 100).toFixed(0)}%`
                  : "Wait for risk to normalize"}
              </div>
            </div>
          </div>
        </div>

        {/* Recommendations */}
        {recommendations.length > 0 && (
          <div className="space-y-1">
            <div className="text-bb-dim text-[9px] uppercase tracking-wider">Recommendations</div>
            {recommendations.map((rec, i) => (
              <div key={i} className="bg-bb-raised border border-bb-border p-1.5 text-[9px]">
                <span className="text-bb-cyan">→</span> {rec}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
