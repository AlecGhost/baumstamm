import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { SerialTaskQueue } from "./serial-task-queue.ts";

const deferred = <T>() => {
  let resolve!: (value: T) => void;
  let reject!: (reason: Error) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
};

describe("SerialTaskQueue", () => {
  it("runs tasks in FIFO order without overlap", async () => {
    const queue = new SerialTaskQueue();
    const firstGate = deferred<void>();
    const events: string[] = [];
    let active = 0;
    let maximumActive = 0;

    const first = queue.enqueue(async () => {
      events.push("first:start");
      active += 1;
      maximumActive = Math.max(maximumActive, active);
      await firstGate.promise;
      active -= 1;
      events.push("first:end");
      return 1;
    });
    const second = queue.enqueue(async () => {
      events.push("second:start");
      active += 1;
      maximumActive = Math.max(maximumActive, active);
      active -= 1;
      events.push("second:end");
      return 2;
    });

    await Promise.resolve();
    assert.deepEqual(events, ["first:start"]);
    firstGate.resolve();

    assert.deepEqual(await Promise.all([first, second]), [1, 2]);
    assert.deepEqual(events, [
      "first:start",
      "first:end",
      "second:start",
      "second:end",
    ]);
    assert.equal(maximumActive, 1);
  });

  it("continues with the next task after a rejection", async () => {
    const queue = new SerialTaskQueue();
    const failure = new Error("expected failure");
    const events: string[] = [];

    const rejected = queue.enqueue(() => {
      events.push("failed");
      throw failure;
    });
    const recovered = queue.enqueue(() => {
      events.push("recovered");
      return "ok";
    });

    await assert.rejects(rejected, failure);
    assert.equal(await recovered, "ok");
    assert.deepEqual(events, ["failed", "recovered"]);
  });
});
