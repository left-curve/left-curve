import { cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useSigningClient } from "../../../store/src/hooks/useSigningClient";
import { createQueryClientWrapper } from "./utils/query-client";

const hookMocks = vi.hoisted(() => ({
  useConnectorClient: vi.fn(),
  useSessionKey: vi.fn(),
}));

vi.mock("../../../store/src/hooks/useConnectorClient.js", () => ({
  useConnectorClient: hookMocks.useConnectorClient,
}));

vi.mock("../../../store/src/hooks/useSessionKey.js", () => ({
  useSessionKey: hookMocks.useSessionKey,
}));

describe("useSigningClient", () => {
  beforeEach(() => {
    hookMocks.useConnectorClient.mockReturnValue({ data: undefined });
    hookMocks.useSessionKey.mockReturnValue({ client: undefined });
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("uses the active connector client when no session client is available", async () => {
    const connectorClient = {
      type: "connector",
      uid: "connector-client",
    };
    hookMocks.useConnectorClient.mockReturnValue({ data: connectorClient });

    const { result } = renderHook(() => useSigningClient(), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() => expect(result.current.data).toBe(connectorClient));
    expect(result.current.isSuccess).toBe(true);
  });

  it("prefers the session client over the connector client", async () => {
    const connectorClient = {
      type: "connector",
      uid: "connector-client",
    };
    const sessionClient = {
      type: "session",
      uid: "session-client",
    };
    hookMocks.useConnectorClient.mockReturnValue({ data: connectorClient });
    hookMocks.useSessionKey.mockReturnValue({ client: sessionClient });

    const { result } = renderHook(() => useSigningClient(), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() => expect(result.current.data).toBe(sessionClient));
    expect(result.current.data).not.toBe(connectorClient);
  });

  it("stays idle when there is no connector or session client", () => {
    const { result } = renderHook(() => useSigningClient(), {
      wrapper: createQueryClientWrapper(),
    });

    expect(result.current.data).toBeUndefined();
    expect(result.current.fetchStatus).toBe("idle");
  });

  it("starts resolving when a session client becomes available after an idle render", async () => {
    const sessionClient = {
      type: "session",
      uid: "late-session-client",
    };
    const { result, rerender } = renderHook(() => useSigningClient(), {
      wrapper: createQueryClientWrapper(),
    });

    expect(result.current.data).toBeUndefined();
    expect(result.current.fetchStatus).toBe("idle");

    hookMocks.useSessionKey.mockReturnValue({
      client: sessionClient,
    });
    rerender();

    await waitFor(() => expect(result.current.data).toBe(sessionClient));
    expect(result.current.isSuccess).toBe(true);
    expect(result.current.fetchStatus).toBe("idle");
  });
});
