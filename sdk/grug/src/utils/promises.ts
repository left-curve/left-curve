/**
 * Wait for a given number of milliseconds.
 *
 * @param ms - The number of milliseconds to wait for.
 * @returns A promise that resolves after the given number of milliseconds.
 */
export async function wait(ms: number) {
  return await new Promise((resolve) => setTimeout(resolve, ms));
}

export type WithTimeoutErrorType = Error;

export function withTimeout<data>(
  fn: ({ signal }: { signal: AbortController["signal"] | null }) => Promise<data>,
  {
    errorInstance = new Error("timed out"),
    timeout,
    signal,
  }: {
    // The error instance to throw when the timeout is reached.
    errorInstance?: Error | undefined;
    // The timeout (in ms).
    timeout: number;
    // Whether or not the timeout should use an abort signal.
    signal?: boolean | undefined;
  },
): Promise<data> {
  return new Promise((resolve, reject) => {
    (async () => {
      let timeoutId!: ReturnType<typeof setTimeout>;
      try {
        const controller = new AbortController();
        if (timeout > 0) {
          timeoutId = setTimeout(() => {
            if (signal) {
              controller.abort();
            } else {
              reject(errorInstance);
            }
          }, timeout) as ReturnType<typeof setTimeout>;
        }
        resolve(await fn({ signal: controller?.signal || null }));
      } catch (err) {
        if ((err as Error)?.name === "AbortError") reject(errorInstance);
        reject(err);
      } finally {
        clearTimeout(timeoutId);
      }
    })();
  });
}

export type WithRetryParameters = {
  // The delay (in ms) between retries.
  delay?: ((config: { count: number; error: Error }) => number) | number | undefined;
  // The max number of times to retry.
  retryCount?: number | undefined;
  // Whether or not to retry when an error is thrown.
  shouldRetry?: ({
    count,
    error,
  }: {
    count: number;
    error: Error;
  }) => Promise<boolean> | boolean;
};

export type WithRetryErrorType = Error;

export function withRetry<data>(
  fn: ({ abort }: { abort: (reason: string) => void }) => () => Promise<data>,
  { delay: delay_ = 100, retryCount = 2, shouldRetry = () => true }: WithRetryParameters = {},
) {
  return new Promise<data>((resolve, reject) => {
    const attemptRetry = async ({ count = 0 } = {}) => {
      const retry = async ({ error }: { error: Error }) => {
        const delay = typeof delay_ === "function" ? delay_({ count, error }) : delay_;
        if (delay) await wait(delay);
        attemptRetry({ count: count + 1 });
      };

      try {
        const data = await fn({ abort: (s) => reject(s) })();
        resolve(data);
      } catch (err) {
        if (count < retryCount && (await shouldRetry({ count, error: err as Error })))
          return retry({ error: err as Error });
        reject(err);
      }
    };
    attemptRetry();
  });
}

export type PromiseWithResolvers<type> = {
  promise: Promise<type>;
  resolve: (value: type | PromiseLike<type>) => void;
  reject: (reason?: unknown) => void;
};

export function withResolvers<type>(): PromiseWithResolvers<type> {
  let resolve: PromiseWithResolvers<type>["resolve"] = () => undefined;
  let reject: PromiseWithResolvers<type>["reject"] = () => undefined;

  const promise = new Promise<type>((resolve_, reject_) => {
    resolve = resolve_;
    reject = reject_;
  });

  return { promise, resolve, reject };
}
