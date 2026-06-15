import { QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type React from "react";

import { ErrorPage } from "../src/components/foundation/ErrorPage";
import { Route as MaintenanceRoute } from "../src/pages/maintenance";
import { createTestQueryClient } from "./utils/query-client";

const healthMocks = vi.hoisted(() => ({
  captureException: vi.fn(),
  navigate: vi.fn(),
}));

vi.mock("@sentry/react", () => ({
  captureException: healthMocks.captureException,
}));

vi.mock("@tanstack/react-router", () => ({
  createFileRoute: () => (options: unknown) => ({ options }),
  useNavigate: () => healthMocks.navigate,
}));

vi.mock("../src/components/foundation/NotFound", () => ({
  NotFound: () => <div data-testid="not-found">not found</div>,
}));

vi.mock("~/components/foundation/Maintenance", () => ({
  Maintenance: () => <div data-testid="maintenance">maintenance mode</div>,
}));

type HealthResponse = boolean | "network-error" | "http-error";

function mockHealthResponses(...responses: HealthResponse[]) {
  const pending = [...responses];
  const fetchMock = vi.fn(async () => {
    const next = pending.shift() ?? responses.at(-1) ?? false;
    if (next === "network-error") throw new Error("network down");
    if (next === "http-error") {
      return {
        ok: false,
        json: vi.fn(),
      };
    }
    return {
      ok: true,
      json: vi.fn().mockResolvedValue({ is_running: next }),
    };
  });
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

function renderWithQueryClient(component: React.ReactNode) {
  const queryClient = createTestQueryClient();
  const rendered = render(
    <QueryClientProvider client={queryClient}>{component}</QueryClientProvider>,
  );
  return { queryClient, ...rendered };
}

function MaintenanceComponent() {
  return (
    MaintenanceRoute as unknown as {
      options: { component: React.ComponentType };
    }
  ).options.component;
}

describe("health-aware routing", () => {
  beforeEach(() => {
    sessionStorage.removeItem("chunk_refresh_timestamp");
    Object.defineProperty(window, "dango", {
      configurable: true,
      value: {
        urls: {
          upUrl: "https://status.example/up",
        },
      },
    });
    window.history.pushState({}, "", "/");
  });

  afterEach(() => {
    cleanup();
    sessionStorage.removeItem("chunk_refresh_timestamp");
    vi.clearAllMocks();
    vi.unstubAllGlobals();
  });

  it("redirects route errors to maintenance when the backend health check is down", async () => {
    const error = new Error("route failed");
    mockHealthResponses(false);

    const { container } = renderWithQueryClient(<ErrorPage error={error} reset={vi.fn()} />);

    expect(container.querySelector(".animate-spinner-ease-spin")).toBeInTheDocument();
    await waitFor(() => {
      expect(healthMocks.navigate).toHaveBeenCalledWith({ to: "/maintenance" });
    });
    expect(healthMocks.captureException).not.toHaveBeenCalled();
  });

  it.each([
    ["HTTP error", "http-error"],
    ["transport error", "network-error"],
  ] as const)(
    "redirects route errors to maintenance when the backend health check has a %s",
    async (_label, response) => {
      const error = new Error("route failed");
      const fetchMock = mockHealthResponses(response);

      renderWithQueryClient(<ErrorPage error={error} reset={vi.fn()} />);

      await waitFor(() => {
        expect(healthMocks.navigate).toHaveBeenCalledWith({ to: "/maintenance" });
      });
      expect(fetchMock).toHaveBeenCalledWith("https://status.example/up");
      expect(healthMocks.captureException).not.toHaveBeenCalled();
    },
  );

  it("captures route errors and renders not-found when the backend is healthy", async () => {
    const error = new Error("route failed");
    mockHealthResponses(true);

    renderWithQueryClient(<ErrorPage error={error} reset={vi.fn()} />);

    expect(await screen.findByTestId("not-found")).toBeInTheDocument();
    expect(healthMocks.captureException).toHaveBeenCalledWith(error);
    expect(healthMocks.navigate).not.toHaveBeenCalled();
  });

  it("handles chunk-load errors with the reload helper and skips the health check", async () => {
    const reset = vi.fn();
    sessionStorage.setItem("chunk_refresh_timestamp", Date.now().toString());
    const fetchMock = mockHealthResponses(true);

    renderWithQueryClient(<ErrorPage error={new Error("Loading chunk 42 failed")} reset={reset} />);

    await waitFor(() => {
      expect(reset).toHaveBeenCalledOnce();
    });
    expect(fetchMock).not.toHaveBeenCalled();
    expect(healthMocks.navigate).not.toHaveBeenCalled();
    expect(healthMocks.captureException).not.toHaveBeenCalled();
  });

  it("requires three consecutive healthy maintenance checks before navigating home", async () => {
    window.history.pushState({}, "", "/maintenance");
    const fetchMock = mockHealthResponses(true, true, true);
    const Component = MaintenanceComponent();
    const { queryClient } = renderWithQueryClient(<Component />);

    expect(await screen.findByTestId("maintenance")).toBeInTheDocument();
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(healthMocks.navigate).not.toHaveBeenCalled();

    await act(async () => {
      await queryClient.refetchQueries({ queryKey: ["maintenance_chain_status"] });
    });
    expect(healthMocks.navigate).not.toHaveBeenCalled();

    await act(async () => {
      await queryClient.refetchQueries({ queryKey: ["maintenance_chain_status"] });
    });
    await waitFor(() => {
      expect(healthMocks.navigate).toHaveBeenCalledWith({ to: "/" });
    });
  });

  it("resets the maintenance recovery counter after a failed health check", async () => {
    window.history.pushState({}, "", "/maintenance");
    mockHealthResponses(true, "http-error", true, true);
    const Component = MaintenanceComponent();
    const { queryClient } = renderWithQueryClient(<Component />);

    expect(await screen.findByTestId("maintenance")).toBeInTheDocument();

    await act(async () => {
      await queryClient.refetchQueries({ queryKey: ["maintenance_chain_status"] });
      await queryClient.refetchQueries({ queryKey: ["maintenance_chain_status"] });
      await queryClient.refetchQueries({ queryKey: ["maintenance_chain_status"] });
    });

    expect(healthMocks.navigate).not.toHaveBeenCalled();
  });

  it("resets the maintenance recovery counter after a thrown health check", async () => {
    window.history.pushState({}, "", "/maintenance");
    mockHealthResponses(true, "network-error", true, true);
    const Component = MaintenanceComponent();
    const { queryClient } = renderWithQueryClient(<Component />);

    expect(await screen.findByTestId("maintenance")).toBeInTheDocument();

    await act(async () => {
      await queryClient.refetchQueries({ queryKey: ["maintenance_chain_status"] });
      await queryClient.refetchQueries({ queryKey: ["maintenance_chain_status"] });
      await queryClient.refetchQueries({ queryKey: ["maintenance_chain_status"] });
    });

    expect(healthMocks.navigate).not.toHaveBeenCalled();
  });

  it("requires consecutive healthy checks after the backend reports not running", async () => {
    window.history.pushState({}, "", "/maintenance");
    const fetchMock = mockHealthResponses(true, false, true, true, true);
    const Component = MaintenanceComponent();
    const { queryClient } = renderWithQueryClient(<Component />);

    expect(await screen.findByTestId("maintenance")).toBeInTheDocument();
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(healthMocks.navigate).not.toHaveBeenCalled();

    await act(async () => {
      await queryClient.refetchQueries({ queryKey: ["maintenance_chain_status"] });
    });
    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(healthMocks.navigate).not.toHaveBeenCalled();

    await act(async () => {
      await queryClient.refetchQueries({ queryKey: ["maintenance_chain_status"] });
    });
    expect(fetchMock).toHaveBeenCalledTimes(3);
    expect(healthMocks.navigate).not.toHaveBeenCalled();

    await act(async () => {
      await queryClient.refetchQueries({ queryKey: ["maintenance_chain_status"] });
    });
    expect(fetchMock).toHaveBeenCalledTimes(4);
    expect(healthMocks.navigate).not.toHaveBeenCalled();

    await act(async () => {
      await queryClient.refetchQueries({ queryKey: ["maintenance_chain_status"] });
    });
    await waitFor(() => {
      expect(healthMocks.navigate).toHaveBeenCalledWith({ to: "/" });
    });
    expect(fetchMock).toHaveBeenCalledTimes(5);
  });
});
