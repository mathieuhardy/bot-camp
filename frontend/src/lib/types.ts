// Mirrors the serde shapes defined in `src/dashboard.rs`, `src/rate_limit/`,
// `src/honeypot.rs` and `src/challenge.rs` on the Rust side.

export type KeyStrategy = 'ip' | 'user_agent' | 'both';

export type Algorithm =
  | { algorithm: 'token_bucket'; capacity: number; refill_per_sec: number }
  | { algorithm: 'fixed_window'; limit: number; window_ms: number }
  | { algorithm: 'sliding_window'; limit: number; window_ms: number }
  | { algorithm: 'min_interval'; min_interval_ms: number };

export type RateLimitConfig = Algorithm & {
  key_strategy: KeyStrategy;
  ban_threshold: number;
  ban_duration_ms: number;
  allow_ips: string[];
  block_ips: string[];
  allow_user_agents: string[];
  block_user_agents: string[];
};

export interface HoneypotConfig {
  key_strategy: KeyStrategy;
  ban_duration_ms: number;
}

export interface ChallengeConfig {
  delay_ms: number;
  cookie_max_age_secs: number;
}

export interface RateLimitKeyEntry {
  key: string;
  banned: boolean;
  retry_after_secs: number | null;
  consecutive_violations: number;
}

export interface HoneypotKeyEntry {
  key: string;
  banned: boolean;
  retry_after_secs: number | null;
}

export interface RateLimitSnapshot {
  config: RateLimitConfig;
  keys: RateLimitKeyEntry[];
}

export interface HoneypotSnapshot {
  config: HoneypotConfig;
  keys: HoneypotKeyEntry[];
}

export interface Snapshot {
  rate_limit: RateLimitSnapshot;
  honeypot: HoneypotSnapshot;
  challenge: ChallengeConfig;
}

export type EventSource = 'rate_limit' | 'honeypot';

// A rate limiter decision is one of "allowed" | "limited" | "banned" |
// "blocked" | "allow_listed"; a honeypot decision is "blocked" | "trapped".
export type Decision = 'allowed' | 'limited' | 'banned' | 'blocked' | 'allow_listed' | 'trapped';

export interface DashboardEvent {
  timestamp_ms: number;
  source: EventSource;
  key: string;
  decision: Decision;
  retry_after_secs: number | null;
}

export type DashboardMessage = ({ type: 'snapshot' } & Snapshot) | ({ type: 'event' } & DashboardEvent);
