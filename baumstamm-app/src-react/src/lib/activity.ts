export type ActivityListener = () => void;

export interface ActivitySource {
  getSnapshot(): boolean;
  subscribe(listener: ActivityListener): () => void;
}

export class ActivityStore implements ActivitySource {
  private activeCount = 0;
  private readonly listeners = new Set<ActivityListener>();

  getSnapshot = () => this.activeCount > 0;

  subscribe = (listener: ActivityListener) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  begin() {
    this.activeCount += 1;
    if (this.activeCount === 1) this.emit();

    let ended = false;
    return () => {
      if (ended) return;
      ended = true;
      this.activeCount = Math.max(0, this.activeCount - 1);
      if (this.activeCount === 0) this.emit();
    };
  }

  private emit() {
    this.listeners.forEach((listener) => listener());
  }
}

type TimerHandle = ReturnType<typeof setTimeout>;

export type TimerScheduler = {
  setTimer(callback: () => void, delayMs: number): TimerHandle;
  clearTimer(handle: TimerHandle): void;
};

const defaultScheduler: TimerScheduler = {
  setTimer: (callback, delayMs) => setTimeout(callback, delayMs),
  clearTimer: (handle) => clearTimeout(handle),
};

export const createDelayedActivitySource = (
  source: ActivitySource,
  delayMs: number,
  scheduler: TimerScheduler = defaultScheduler,
): ActivitySource & { dispose(): void } => {
  let visible = false;
  let timer: TimerHandle | null = null;
  const listeners = new Set<ActivityListener>();

  const emit = () => listeners.forEach((listener) => listener());
  const clearPendingTimer = () => {
    if (timer === null) return;
    scheduler.clearTimer(timer);
    timer = null;
  };
  const update = () => {
    if (source.getSnapshot()) {
      if (visible || timer !== null) return;
      timer = scheduler.setTimer(() => {
        timer = null;
        if (!source.getSnapshot() || visible) return;
        visible = true;
        emit();
      }, delayMs);
      return;
    }

    clearPendingTimer();
    if (!visible) return;
    visible = false;
    emit();
  };

  const unsubscribe = source.subscribe(update);
  update();

  return {
    getSnapshot: () => visible,
    subscribe: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    dispose: () => {
      unsubscribe();
      clearPendingTimer();
      listeners.clear();
    },
  };
};
