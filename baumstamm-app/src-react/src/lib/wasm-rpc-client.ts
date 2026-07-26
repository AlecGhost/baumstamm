import { ActivityStore } from "./activity";
import { SerialTaskQueue } from "./serial-task-queue";
import type {
  WasmRpcMethod,
  WasmRpcMethods,
  WasmRpcRequest,
  WasmRpcResponse,
} from "./wasm-rpc";

type PendingRequest = {
  resolve(value: unknown): void;
  reject(error: Error): void;
};

const workerError = (message: string) =>
  new Error(`WASM worker failed: ${message}`);

export class WasmRpcClient {
  readonly activity = new ActivityStore();

  private readonly queue = new SerialTaskQueue();
  private readonly pending = new Map<number, PendingRequest>();
  private readonly createWorker: () => Worker;
  private worker: Worker | null = null;
  private nextRequestId = 1;

  constructor(createWorker: () => Worker) {
    this.createWorker = createWorker;
  }

  call<Method extends WasmRpcMethod>(
    method: Method,
    ...args: WasmRpcMethods[Method]["args"]
  ): Promise<WasmRpcMethods[Method]["result"]> {
    const endActivity = this.activity.begin();
    return this.queue
      .enqueue(() => this.dispatch(method, args))
      .finally(endActivity);
  }

  dispose() {
    this.failPending(workerError("worker was disposed"));
    this.worker?.terminate();
    this.worker = null;
  }

  private dispatch<Method extends WasmRpcMethod>(
    method: Method,
    args: WasmRpcMethods[Method]["args"],
  ): Promise<WasmRpcMethods[Method]["result"]> {
    const id = this.nextRequestId++;
    const request = { id, method, args } as WasmRpcRequest;
    const worker = this.getWorker();

    return new Promise<WasmRpcMethods[Method]["result"]>((resolve, reject) => {
      this.pending.set(id, {
        resolve: (value) => resolve(value as WasmRpcMethods[Method]["result"]),
        reject,
      });
      try {
        worker.postMessage(request);
      } catch (error) {
        this.pending.delete(id);
        reject(
          workerError(error instanceof Error ? error.message : String(error)),
        );
      }
    });
  }

  private getWorker() {
    if (this.worker) return this.worker;

    const worker = this.createWorker();
    worker.addEventListener("message", this.handleMessage);
    worker.addEventListener("error", this.handleWorkerError);
    worker.addEventListener("messageerror", this.handleMessageError);
    this.worker = worker;
    return worker;
  }

  private readonly handleMessage = (event: MessageEvent<WasmRpcResponse>) => {
    const response = event.data;
    const request = this.pending.get(response.id);
    if (!request) return;
    this.pending.delete(response.id);

    if (response.ok) {
      request.resolve(response.value);
      return;
    }

    const error = new Error(response.error.message);
    error.name = response.error.name;
    if (response.error.stack) error.stack = response.error.stack;
    request.reject(error);
  };

  private readonly handleWorkerError = (event: ErrorEvent) => {
    this.resetWorker(workerError(event.message || "unknown runtime error"));
  };

  private readonly handleMessageError = () => {
    this.resetWorker(workerError("could not deserialize a worker response"));
  };

  private resetWorker(error: Error) {
    this.failPending(error);
    this.worker?.terminate();
    this.worker = null;
  }

  private failPending(error: Error) {
    this.pending.forEach(({ reject }) => reject(error));
    this.pending.clear();
  }
}
