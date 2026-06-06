import { act, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { createLiveResource } from "../../../store/src/live/createLiveResource";
import { useLiveResource } from "../../../store/src/live/useLiveResource";
import { subscriptionsStore } from "../../../store/src/subscriptions";

import type { LiveResourceSnapshot } from "../../../store/src/live/types";
import type { LiveResourceEmitOptions } from "../../../store/src/live/types";

type TestSnapshot = LiveResourceSnapshot & {
  value: number;
};

const initialSnapshot: TestSnapshot = {
  status: "idle",
  error: null,
  value: 0,
};

describe("createLiveResource", () => {
  it("starts once for duplicate acquires and stops after the final release", () => {
    let starts = 0;
    let stops = 0;

    const resource = createLiveResource<{ id: string }, TestSnapshot>({
      name: "primitiveDuplicateAcquire",
      getKey: ({ id }) => id,
      getInitialSnapshot: () => initialSnapshot,
      start: () => {
        starts += 1;
        return () => {
          stops += 1;
        };
      },
    });

    const releaseA = resource.acquire({ id: "a" });
    const releaseB = resource.acquire({ id: "a" });

    expect(starts).toBe(1);
    expect(stops).toBe(0);

    releaseA();
    expect(stops).toBe(0);

    releaseB();
    expect(stops).toBe(1);
    expect(resource.getDebugState().entries).toHaveLength(0);
  });

  it("isolates keys, guards stale versions, applies equality, and honors cache policy", () => {
    const emitters = new Map<
      string,
      (snapshot: TestSnapshot, options?: LiveResourceEmitOptions) => void
    >();
    const updates: number[] = [];

    const resource = createLiveResource<{ id: string }, TestSnapshot>({
      name: "primitiveVersionEquality",
      cache: "keep",
      getKey: ({ id }) => id,
      getInitialSnapshot: () => initialSnapshot,
      equal: (previous, next) =>
        previous.status === next.status &&
        previous.error === next.error &&
        previous.value === next.value,
      start: (params, { emit }) => {
        emitters.set(params.id, emit);
        return () => {};
      },
    });

    const releaseA = resource.acquire({ id: "a" });
    resource.acquire({ id: "b" });
    resource.subscribe({ id: "a" }, () => updates.push(resource.getSnapshot({ id: "a" }).value));

    emitters.get("a")?.({ status: "ready", error: null, value: 1 }, { version: 2 });
    emitters.get("a")?.({ status: "ready", error: null, value: 2 }, { version: 1 });
    emitters.get("a")?.({ status: "ready", error: null, value: 1 }, { version: 3 });
    emitters.get("b")?.({ status: "ready", error: null, value: 9 }, { version: 1 });

    expect(resource.getSnapshot({ id: "a" }).value).toBe(1);
    expect(resource.getSnapshot({ id: "b" }).value).toBe(9);
    expect(updates).toEqual([1]);

    releaseA();
    expect(resource.getSnapshot({ id: "a" }).value).toBe(1);
  });

  it("stores runtime errors on the latest snapshot and notifies subscribers", () => {
    let reportError: ((error: unknown) => void) | undefined;
    const updates: TestSnapshot[] = [];

    const resource = createLiveResource<{ id: string }, TestSnapshot>({
      name: "primitiveRuntimeError",
      cache: "keep",
      getKey: ({ id }) => id,
      getInitialSnapshot: () => initialSnapshot,
      start: (_params, context) => {
        context.emit({ status: "ready", error: null, value: 7 });
        reportError = context.error;
        return () => {};
      },
    });

    const release = resource.acquire({ id: "a" });
    const unsubscribe = resource.subscribe({ id: "a" }, () => {
      updates.push(resource.getSnapshot({ id: "a" }));
    });

    const runtimeError = new Error("boom");
    reportError?.(runtimeError);

    expect(resource.getSnapshot({ id: "a" })).toEqual({
      status: "error",
      error: runtimeError,
      value: 7,
    });
    expect(updates).toEqual([{ status: "error", error: runtimeError, value: 7 }]);

    unsubscribe();
    release();
  });

  it("logs and ignores late errors after delete-on-release eviction", () => {
    let reportError: ((error: unknown) => void) | undefined;
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});

    const resource = createLiveResource<{ id: string }, TestSnapshot>({
      name: "primitiveLateError",
      getKey: ({ id }) => id,
      getInitialSnapshot: () => initialSnapshot,
      start: (_params, context) => {
        reportError = context.error;
        return () => {};
      },
    });

    const release = resource.acquire({ id: "a" });
    release();

    const lateError = new Error("late");
    reportError?.(lateError);

    expect(consoleError).toHaveBeenCalledWith(
      "[live-resource:primitiveLateError] dropped error after release",
      lateError,
    );
    expect(resource.getDebugState().entries).toHaveLength(0);

    consoleError.mockRestore();
  });

  it("supports key-only subscriptions for existing entries without creating missing entries", () => {
    let emit: ((snapshot: TestSnapshot) => void) | undefined;
    const listener = vi.fn();

    const resource = createLiveResource<{ id: string }, TestSnapshot>({
      name: "primitiveKeyOnlySubscribe",
      getKey: ({ id }) => id,
      getInitialSnapshot: () => initialSnapshot,
      start: (_params, context) => {
        emit = context.emit;
        return () => {};
      },
    });

    const unsubscribeMissing = resource.subscribeKey("missing", listener);
    unsubscribeMissing();

    expect(resource.getDebugState().entries).toHaveLength(0);

    const release = resource.acquire({ id: "a" });
    const unsubscribe = resource.subscribeKey("a", listener);

    emit?.({ status: "ready", error: null, value: 3 });

    expect(listener).toHaveBeenCalledTimes(1);

    unsubscribe();
    release();
  });
});

describe("useLiveResource", () => {
  it("returns the idle snapshot and does not start when disabled", () => {
    let starts = 0;

    const resource = createLiveResource<{ id: string }, TestSnapshot>({
      name: "reactDisabled",
      getKey: ({ id }) => id,
      getInitialSnapshot: () => initialSnapshot,
      start: () => {
        starts += 1;
        return () => {};
      },
    });

    function Consumer() {
      const value = useLiveResource({
        resource,
        params: { id: "" },
        enabled: false,
        selector: (snapshot) => snapshot.value,
      });

      return <div data-testid="disabled">{value}</div>;
    }

    render(<Consumer />);

    expect(screen.getByTestId("disabled")).toHaveTextContent("0");
    expect(starts).toBe(0);
  });

  it("shares one underlying resource across duplicate consumers", async () => {
    let starts = 0;
    let stops = 0;
    let emit: ((snapshot: TestSnapshot) => void) | undefined;

    const resource = createLiveResource<{ id: string }, TestSnapshot>({
      name: "reactDuplicateConsumers",
      getKey: ({ id }) => id,
      getInitialSnapshot: () => initialSnapshot,
      start: (_params, context) => {
        starts += 1;
        emit = context.emit;
        return () => {
          stops += 1;
        };
      },
    });

    function Consumer({ label }: { label: string }) {
      const value = useLiveResource({
        resource,
        params: { id: "shared" },
        enabled: true,
        selector: (snapshot) => snapshot.value,
      });

      return <div data-testid={label}>{value}</div>;
    }

    const rendered = render(
      <>
        <Consumer label="a" />
        <Consumer label="b" />
      </>,
    );

    await waitFor(() => expect(starts).toBe(1));

    act(() => {
      emit?.({ status: "ready", error: null, value: 7 });
    });

    expect(screen.getByTestId("a")).toHaveTextContent("7");
    expect(screen.getByTestId("b")).toHaveTextContent("7");

    rendered.rerender(<Consumer label="a" />);
    expect(stops).toBe(0);

    rendered.unmount();
    expect(stops).toBe(1);
  });

  it("restarts same-key resources when the restart token changes", async () => {
    const starts: string[] = [];
    const stops: string[] = [];
    const emitters = new Map<string, (snapshot: TestSnapshot) => void>();

    const resource = createLiveResource<{ id: string; source: string }, TestSnapshot>({
      name: "reactRestartToken",
      getKey: ({ id }) => id,
      getInitialSnapshot: () => initialSnapshot,
      start: (params, context) => {
        starts.push(params.source);
        emitters.set(params.source, context.emit);
        return () => {
          stops.push(params.source);
        };
      },
    });

    function Consumer({ source }: { source: string }) {
      const value = useLiveResource({
        resource,
        params: { id: "shared", source },
        enabled: true,
        selector: (snapshot) => snapshot.value,
        restartToken: source,
      });

      return <div data-testid="restart">{value}</div>;
    }

    const rendered = render(<Consumer source="a" />);
    await waitFor(() => expect(starts).toEqual(["a"]));

    act(() => {
      emitters.get("a")?.({ status: "ready", error: null, value: 1 });
    });
    expect(screen.getByTestId("restart")).toHaveTextContent("1");

    rendered.rerender(<Consumer source="b" />);
    await waitFor(() => expect(starts).toEqual(["a", "b"]));
    expect(stops).toEqual(["a"]);

    act(() => {
      emitters.get("b")?.({ status: "ready", error: null, value: 2 });
    });
    expect(screen.getByTestId("restart")).toHaveTextContent("2");

    rendered.unmount();
    expect(stops).toEqual(["a", "b"]);
  });

  it("starts and stops when enabled toggles", async () => {
    let starts = 0;
    let stops = 0;

    const resource = createLiveResource<{ id: string }, TestSnapshot>({
      name: "reactEnabledToggle",
      getKey: ({ id }) => id,
      getInitialSnapshot: () => initialSnapshot,
      start: () => {
        starts += 1;
        return () => {
          stops += 1;
        };
      },
    });

    function Consumer({ enabled }: { enabled: boolean }) {
      const value = useLiveResource({
        resource,
        params: { id: "toggle" },
        enabled,
        selector: (snapshot) => snapshot.value,
      });

      return <div data-testid="toggle">{value}</div>;
    }

    const rendered = render(<Consumer enabled={false} />);

    expect(screen.getByTestId("toggle")).toHaveTextContent("0");
    expect(starts).toBe(0);

    rendered.rerender(<Consumer enabled />);
    await waitFor(() => expect(starts).toBe(1));

    rendered.rerender(<Consumer enabled={false} />);
    await waitFor(() => expect(stops).toBe(1));

    rendered.rerender(<Consumer enabled />);
    await waitFor(() => expect(starts).toBe(2));

    rendered.unmount();
    expect(stops).toBe(2);
  });
});

describe("subscriptionsStore", () => {
  it("removes listener state when executor startup throws", () => {
    const startupError = new Error("startup failed");
    let calls = 0;
    let emitRuntimeError: ((error: unknown) => void) | undefined;

    const store = subscriptionsStore({
      blockSubscription: ({ error }: { error: (error: unknown) => void }) => {
        calls += 1;
        if (calls === 1) throw startupError;
        emitRuntimeError = error;
        return () => {};
      },
    } as never);

    const firstErrorListener = vi.fn();
    expect(() =>
      store.subscribe("block", {
        listener: vi.fn(),
        onError: firstErrorListener,
      }),
    ).toThrow(startupError);

    const secondErrorListener = vi.fn();
    const unsubscribe = store.subscribe("block", {
      listener: vi.fn(),
      onError: secondErrorListener,
    });

    emitRuntimeError?.(new Error("runtime failed"));

    expect(firstErrorListener).not.toHaveBeenCalled();
    expect(secondErrorListener).toHaveBeenCalledTimes(1);

    unsubscribe();
  });

  it("routes listener exceptions through global and subscription error handlers", () => {
    let emitBlock: ((event: unknown) => void) | undefined;
    const globalError = vi.fn();

    const store = subscriptionsStore(
      {
        blockSubscription: ({ next }: { next: (event: unknown) => void }) => {
          emitBlock = next;
          return () => {};
        },
      } as never,
      { onError: globalError },
    );

    const listenerError = new Error("listener failed");
    const listenerOnError = vi.fn();

    store.subscribe("block", {
      listener: () => {
        throw listenerError;
      },
      onError: listenerOnError,
    });

    emitBlock?.({ block: { height: 1 } });

    expect(globalError).toHaveBeenCalledWith(listenerError);
    expect(listenerOnError).toHaveBeenCalledWith(listenerError);
  });

  it("routes emit listener exceptions through global and subscription error handlers", () => {
    const globalError = vi.fn();
    const listenerError = new Error("emit listener failed");
    const listenerOnError = vi.fn();

    const store = subscriptionsStore({} as never, { onError: globalError });

    store.subscribe("submitTx", {
      listener: () => {
        throw listenerError;
      },
      onError: listenerOnError,
    });

    store.emit({ key: "submitTx" }, { status: "pending" });

    expect(globalError).toHaveBeenCalledWith(listenerError);
    expect(listenerOnError).toHaveBeenCalledWith(listenerError);
  });
});
