// TypeScript interfaces mirroring Rust structs from crates/simmons-cli/src/web.rs

export interface DashboardUpdate {
  timestamp: number;
  layers: LayersData;
}

export interface LayersData {
  data_ingestion: DataIngestionLayer;
  ai_intelligence: AIIntelligenceLayer;
  decision_risk: DecisionRiskLayer;
  execution: ExecutionLayer;
  infrastructure: InfrastructureLayer;
  feedback: FeedbackData;
}

// Data Ingestion Layer
export interface DataIngestionLayer {
  okx_status: FeedStatus;
  xlayer_status: FeedStatus;
  nunchi_status: FeedStatus;
  news_status: FeedStatus;
  symbols: SymbolData[];
  price_history: Record<string, PricePoint[]>;
}

export interface FeedStatus {
  connected: boolean;
  last_update: number;
  message_count: number;
  latency_ms: number;
}

export interface SymbolData {
  symbol: string;
  price: string;
  bid: string;
  ask: string;
  spread_bps: string;
  volume_24h: string;
  change_24h: string;
}

export interface PricePoint {
  time: number;
  price: number;
  volume: number;
}

// AI Intelligence Layer
export interface AIIntelligenceLayer {
  strategy_signals: StrategySignalData[];
  regime: RegimeData;
  nunchi_score: NunchiScoreData;
  forecasts: ForecastData[];
  patterns: PatternData[];
  autoresearch: AutoresearchData;
}

export interface StrategySignalData {
  symbol: string;
  strategy: string;
  signal: string;
  confidence: number;
  reason: string;
}

export interface RegimeData {
  current: string;
  volatility: number;
  trend_strength: number;
  regime_age_mins: number;
}

export interface NunchiScoreData {
  score: number;
  direction: string;
  confidence: number;
  should_trade: boolean;
  components: Record<string, number>;
}

export interface ForecastData {
  symbol: string;
  horizon: string;
  direction: string;
  predicted_change_pct: number;
  confidence: number;
}

export interface PatternData {
  name: string;
  pattern_type: string;
  win_rate: number;
  occurrences: number;
  active: boolean;
}

export interface AutoresearchData {
  active_hypotheses: number;
  patterns_discovered: number;
  last_discovery: string | null;
  alpha_score: number;
}

// Decision & Risk Layer
export interface DecisionRiskLayer {
  portfolio: PortfolioData;
  positions: PositionData[];
  rebalancer: RebalancerData;
  arb_opportunities: ArbOpportunityData[];
  risk_metrics: RiskMetricsData;
  kelly_sizing: KellySizingData;
}

export interface PortfolioData {
  capital: number;
  equity: number;
  pnl: number;
  pnl_pct: number;
  drawdown: number;
  max_drawdown: number;
  sharpe_ratio: number;
  win_rate: number;
  total_trades: number;
}

export interface PositionData {
  symbol: string;
  side: string;
  size: number;
  entry_price: number;
  current_price: number;
  pnl: number;
  pnl_pct: number;
  stop_loss: number | null;
  take_profit: number | null;
}

export interface RebalancerData {
  target_weights: Record<string, number>;
  current_weights: Record<string, number>;
  drift_pct: number;
  rebalance_needed: boolean;
  pending_trades: number;
}

export interface ArbOpportunityData {
  id: string;
  route: string;
  spread_bps: number;
  expected_profit: number;
  confidence: number;
  expires_in_secs: number;
}

export interface RiskMetricsData {
  var_95: number;
  var_99: number;
  position_limit_used: number;
  daily_loss_limit_used: number;
  correlation_risk: number;
  leverage: number;
}

export interface KellySizingData {
  optimal_fraction: number;
  recommended_size_pct: number;
  edge: number;
  win_prob: number;
}

// Execution Layer
export interface ExecutionLayer {
  router: RouterData;
  mev_shield: MevShieldData;
  gas: GasData;
  pending_orders: PendingOrderData[];
  recent_executions: ExecutionData[];
}

export interface RouterData {
  active_venues: string[];
  best_venue: string;
  split_enabled: boolean;
  avg_slippage_bps: number;
}

export interface MevShieldData {
  enabled: boolean;
  private_pool: string;
  protected_txns: number;
  mev_saved_usd: number;
  current_risk: string;
}

export interface GasData {
  current_gwei: number;
  recommended_gwei: number;
  priority: string;
  estimated_cost_usd: number;
  should_wait: boolean;
}

export interface PendingOrderData {
  id: string;
  symbol: string;
  side: string;
  size: number;
  status: string;
  venue: string;
}

export interface ExecutionData {
  id: string;
  symbol: string;
  side: string;
  size: number;
  price: number;
  slippage_bps: number;
  time: number;
}

// Infrastructure Layer
export interface InfrastructureLayer {
  xlayer: XLayerData;
  bridge: BridgeData;
  dex_pools: DexPoolData[];
  cod3x: Cod3xData;
}

export interface XLayerData {
  connected: boolean;
  block_number: number;
  gas_price_gwei: number;
  tps: number;
}

export interface BridgeData {
  l1_balance: number;
  l2_balance: number;
  pending_deposits: number;
  pending_withdrawals: number;
  avg_bridge_time_mins: number;
}

export interface DexPoolData {
  name: string;
  pair: string;
  liquidity_usd: number;
  volume_24h: number;
  apr: number;
}

export interface Cod3xData {
  connected: boolean;
  total_deposited: number;
  total_borrowed: number;
  health_factor: number;
  available_to_borrow: number;
  liquidation_risk: string;
}

// Feedback Layer
export interface FeedbackData {
  learning_enabled: boolean;
  trades_recorded: number;
  insights: string[];
  strategy_adjustments: Record<string, number>;
  pattern_effectiveness: Record<string, number>;
}

// ===== SIMMONS DUAL BRAIN v3.0 =====

// TA Brain (Nunchi 14 Strats)
export interface TABrainData {
  radar_score: number; // 0-200
  pulse_tier: number; // 0-6
  guard: GuardData;
  apex: ApexData;
  strategies: StrategyCategories;
}

export interface GuardData {
  phase: number; // 1 or 2
  active_stops: number;
  triggered: boolean;
}

export interface ApexData {
  concurrent_slots: number; // max 4
  active_slots: number;
  priority_tier: string;
  roe_exit_active: boolean;
}

export interface StrategyCategories {
  mm: StrategyStatus[]; // engine, avellaneda, regime, grid, liq
  arb: StrategyStatus[]; // funding, basis
  dir: StrategyStatus[]; // momentum, mean_rev
  int: StrategyStatus[]; // hedge, rfe, claude
}

export interface StrategyStatus {
  name: string;
  active: boolean;
  signal: string; // buy | sell | hold | skip
  confidence: number;
}

// Fundamental Brain (Multi-Source)
export interface FundBrainData {
  whale_sentiment: number; // -1 to 1
  twitter_sentiment: number; // -1 to 1
  news_sentiment: number; // -1 to 1
  security_flags: string[];
  whale_tracker: WhaleTrackerData;
  news_headlines: NewsHeadline[];
  trending_tokens: string[];
}

export interface WhaleTrackerData {
  smart_money_signals: number;
  kol_tracking: boolean;
  recent_moves: string[];
}

export interface NewsHeadline {
  source: string;
  text: string;
  sentiment: "positive" | "negative" | "neutral";
  time: string;
}

// Consensus Layer
export interface ConsensusData {
  merged_score: number; // combined TA + Fund score
  ta_weight: number; // 0.6
  fund_weight: number; // 0.4
  radar_met: boolean; // RADAR > 170
  fund_positive: boolean;
  entry_approved: boolean;
  conflict: boolean; // triggers Claude debate
  reflect_adjustments: Record<string, number>;
}

// Claude Orchestrator
export interface OrchestratorData {
  debate: DebateData;
  selected_strategy: string; // MM | ARB | Directional
  guard_synced: boolean;
  action: string; // long | short | skip
}

export interface DebateData {
  bull_thesis: string;
  bear_thesis: string;
  risk_assessment: string;
  verdict: string;
}

// Execution Status
export interface ExecutionModeData {
  mode: string; // paper | live_dex | live_perps
  paper: { active: boolean; engine: string };
  live_dex: { active: boolean; chains: string[]; router: string };
  live_perps: { active: boolean; venue: string };
}
