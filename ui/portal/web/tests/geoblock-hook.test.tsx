import { cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { useGeoblock } from "../src/components/foundation/hooks/useGeoblock";
import { createQueryClientWrapper } from "./utils/query-client";

function mockTraceResponse(body: string, ok = true) {
  const fetchMock = vi.fn().mockResolvedValue({
    ok,
    text: vi.fn().mockResolvedValue(body),
  });
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

function mockTraceRejection() {
  const fetchMock = vi.fn().mockRejectedValue(new Error("trace unavailable"));
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

describe("useGeoblock", () => {
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it("blocks users when Cloudflare trace resolves to a restricted country", async () => {
    const fetchMock = mockTraceResponse("ip=203.0.113.1\nloc=US\ncolo=IAD\n");
    const { result } = renderHook(() => useGeoblock(), {
      wrapper: createQueryClientWrapper(),
    });

    expect(result.current).toBe(false);

    await waitFor(() => {
      expect(result.current).toBe(true);
    });
    expect(fetchMock).toHaveBeenCalledWith("/cdn-cgi/trace");
  });

  it("does not block users from unrestricted countries", async () => {
    const fetchMock = mockTraceResponse("ip=203.0.113.2\nloc=PT\ncolo=LIS\n");
    const { result } = renderHook(() => useGeoblock(), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledOnce();
    });
    expect(result.current).toBe(false);
  });

  it("fails open when the trace request fails or omits the country", async () => {
    const fetchMock = mockTraceResponse("ip=203.0.113.3\ncolo=IAD\n", false);
    const { result } = renderHook(() => useGeoblock(), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledOnce();
    });
    expect(result.current).toBe(false);
  });

  it("fails open when the trace request is rejected", async () => {
    const fetchMock = mockTraceRejection();
    const { result } = renderHook(() => useGeoblock(), {
      wrapper: createQueryClientWrapper(),
    });

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledOnce();
    });
    expect(result.current).toBe(false);
  });
});
