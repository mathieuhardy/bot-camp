import type {
  ChallengeConfig,
  DashboardEvent,
  DashboardMessage,
  HoneypotConfig,
  HoneypotKeyEntry,
  RateLimitConfig,
  RateLimitKeyEntry,
} from './types';

const MAX_EVENTS = 100;

class DashboardStore {
  connected = $state(false);
  rateLimitConfig = $state<RateLimitConfig | null>(null);
  honeypotConfig = $state<HoneypotConfig | null>(null);
  challengeConfig = $state<ChallengeConfig | null>(null);
  rateLimitKeys = $state<Map<string, RateLimitKeyEntry>>(new Map());
  honeypotKeys = $state<Map<string, HoneypotKeyEntry>>(new Map());
  events = $state<DashboardEvent[]>([]);

  applyMessage(message: DashboardMessage) {
    if (message.type === 'snapshot') {
      this.rateLimitConfig = message.rate_limit.config;
      this.honeypotConfig = message.honeypot.config;
      this.challengeConfig = message.challenge;
      this.rateLimitKeys = new Map(message.rate_limit.keys.map((entry) => [entry.key, entry]));
      this.honeypotKeys = new Map(message.honeypot.keys.map((entry) => [entry.key, entry]));
      return;
    }

    this.applyEvent(message);
  }

  private applyEvent(event: DashboardEvent) {
    this.events = [event, ...this.events].slice(0, MAX_EVENTS);

    if (event.source === 'rate_limit') {
      this.upsertRateLimitKey(event);
    } else {
      this.upsertHoneypotKey(event);
    }
  }

  private upsertRateLimitKey(event: DashboardEvent) {
    const previous = this.rateLimitKeys.get(event.key);
    const consecutiveViolations =
      event.decision === 'allowed' || event.decision === 'allow_listed'
        ? 0
        : event.decision === 'limited' || event.decision === 'banned'
          ? (previous?.consecutive_violations ?? 0) + 1
          : previous?.consecutive_violations ?? 0;

    const keys = new Map(this.rateLimitKeys);
    keys.set(event.key, {
      key: event.key,
      banned: event.decision === 'banned',
      retry_after_secs: event.retry_after_secs,
      consecutive_violations: consecutiveViolations,
    });
    this.rateLimitKeys = keys;
  }

  private upsertHoneypotKey(event: DashboardEvent) {
    const keys = new Map(this.honeypotKeys);
    keys.set(event.key, {
      key: event.key,
      banned: event.decision === 'trapped' || event.decision === 'blocked',
      retry_after_secs: event.retry_after_secs,
    });
    this.honeypotKeys = keys;
  }
}

export const dashboard = new DashboardStore();
