import { describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

type InspectableRoute = {
  options: {
    component?: unknown;
    head?: () => {
      meta: Array<{
        title: string;
      }>;
    };
  };
  routePath: string;
};

function createInspectableRoute(routePath: string) {
  return (options: InspectableRoute["options"]) => ({
    options,
    routePath,
  });
}

vi.mock("@tanstack/react-router", () => ({
  createFileRoute: createInspectableRoute,
}));

vi.mock("~/components/landing/Landing", () => ({
  Landing: () => <main data-testid="landing" />,
}));

vi.mock("~/components/foundation/NotFound", () => ({
  NotFound: () => <main data-testid="not-found" />,
}));

async function loadRoute(loader: () => Promise<{ Route: unknown }>) {
  const { Route } = await loader();
  return Route as InspectableRoute;
}

describe("route metadata", () => {
  it.each([
    {
      loader: () => import("../src/pages/(app)/_app.index"),
      path: "/(app)/_app/",
      title: `Dango | ${m["common.overview"]()}`,
    },
    {
      loader: () => import("../src/pages/(app)/_app.account.create"),
      path: "/(app)/_app/account/create",
      title: `Dango | ${m["signup.createAccount"]()}`,
    },
    {
      loader: () => import("../src/pages/(app)/_app.settings"),
      path: "/(app)/_app/settings",
      title: `Dango | ${m["settings.title"]()}`,
    },
    {
      loader: () => import("../src/pages/(app)/_app.bridge"),
      path: "/(app)/_app/bridge",
      title: `Dango | ${m["bridge.title"]()}`,
    },
    {
      loader: () => import("../src/pages/(app)/_app.earn.index"),
      path: "/(app)/_app/earn/",
      title: `Dango | ${m["vaultLiquidity.title"]()}`,
    },
    {
      loader: () => import("../src/pages/(app)/_app.transfer"),
      path: "/(app)/_app/transfer",
      title: `Dango | ${m["sendAndReceive.title"]()}`,
    },
    {
      loader: () => import("../src/pages/(app)/_app.trade.$pairSymbols"),
      path: "/(app)/_app/trade/$pairSymbols",
      title: `Dango | ${m["applets.trade.title"]()}`,
    },
    {
      loader: () => import("../src/pages/(app)/_app.account.$address"),
      path: "/(app)/_app/account/$address",
      title: `Dango | ${m["explorer.accounts.title"]()}`,
    },
    {
      loader: () => import("../src/pages/(app)/_app.contract.$address"),
      path: "/(app)/_app/contract/$address",
      title: `Dango | ${m["explorer.contracts.title"]()}`,
    },
    {
      loader: () => import("../src/pages/(app)/_app.block.$block"),
      path: "/(app)/_app/block/$block",
      title: `Dango | ${m["explorer.block.title"]()}`,
    },
    {
      loader: () => import("../src/pages/(app)/_app.tx.$txHash"),
      path: "/(app)/_app/tx/$txHash",
      title: `Dango | ${m["explorer.txs.title"]()}`,
    },
  ])("exposes translated head metadata for $path", async ({ loader, path, title }) => {
    const route = await loadRoute(loader);

    expect(route.routePath).toBe(path);
    expect(route.options.head?.().meta).toEqual([{ title }]);
  });

  it("wires the app catch-all route to the not-found component", async () => {
    const route = await loadRoute(() => import("../src/pages/(app)/_app.$"));

    expect(route.routePath).toBe("/(app)/_app/$");
    expect(route.options.component).toBeTypeOf("function");
  });
});
