type BatchEntry = {
  pollFn: () => Promise<void>;
  interval: number;
  lastRun: number;
};

/**
 * Consolidates multiple independent HTTP polling loops into a single
 * coordinated `setInterval`. Each registered entry fires only when
 * its own interval has elapsed since its last run.
 */
class BatchPoller {
  private entries = new Map<string, BatchEntry>();
  private timer: ReturnType<typeof setInterval> | null = null;
  private tickInterval = 0;

  register(id: string, pollFn: () => Promise<void>, interval: number): void {
    this.entries.set(id, { pollFn, interval, lastRun: 0 });
    this.recalculate();
  }

  unregister(id: string): void {
    this.entries.delete(id);
    this.recalculate();
  }

  private recalculate(): void {
    if (this.timer !== null) {
      clearInterval(this.timer);
      this.timer = null;
    }

    if (this.entries.size === 0) return;

    this.tickInterval = Math.min(...[...this.entries.values()].map((e) => e.interval));
    this.timer = setInterval(() => this.tick(), this.tickInterval);
  }

  private tick(): void {
    const now = Date.now();
    for (const entry of this.entries.values()) {
      if (now - entry.lastRun >= entry.interval) {
        entry.lastRun = now;
        entry.pollFn();
      }
    }
  }
}

export const batchPoller = new BatchPoller();
