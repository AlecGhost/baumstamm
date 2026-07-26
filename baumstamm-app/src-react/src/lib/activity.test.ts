import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  ActivityStore,
  createDelayedActivitySource,
  type TimerScheduler,
} from "./activity.ts";

const createFakeScheduler = () => {
  let nextId = 1;
  const pending = new Map<number, () => void>();
  const scheduler: TimerScheduler = {
    setTimer(callback) {
      const id = nextId++;
      pending.set(id, callback);
      return id as unknown as ReturnType<typeof setTimeout>;
    },
    clearTimer(handle) {
      pending.delete(handle as unknown as number);
    },
  };
  return {
    scheduler,
    flush: () => {
      const callbacks = [...pending.values()];
      pending.clear();
      callbacks.forEach((callback) => callback());
    },
    pendingCount: () => pending.size,
  };
};

describe("delayed activity", () => {
  it("does not show activity that completes before the delay", () => {
    const activity = new ActivityStore();
    const timers = createFakeScheduler();
    const delayed = createDelayedActivitySource(
      activity,
      200,
      timers.scheduler,
    );
    const end = activity.begin();

    assert.equal(delayed.getSnapshot(), false);
    assert.equal(timers.pendingCount(), 1);
    end();
    assert.equal(timers.pendingCount(), 0);
    timers.flush();
    assert.equal(delayed.getSnapshot(), false);
    delayed.dispose();
  });

  it("becomes visible after the delay and hides when work ends", () => {
    const activity = new ActivityStore();
    const timers = createFakeScheduler();
    const delayed = createDelayedActivitySource(
      activity,
      200,
      timers.scheduler,
    );
    let notifications = 0;
    delayed.subscribe(() => {
      notifications += 1;
    });
    const end = activity.begin();

    timers.flush();
    assert.equal(delayed.getSnapshot(), true);
    end();
    assert.equal(delayed.getSnapshot(), false);
    assert.equal(notifications, 2);
    delayed.dispose();
  });
});
