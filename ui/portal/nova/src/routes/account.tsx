import { createRoute, redirect } from "@tanstack/react-router";
import { Route as rootRoute } from "./__root";
import { AccountLayout } from "../account/AccountLayout";
import { Overview } from "../account/Overview";
import { Portfolio } from "../account/Portfolio";
import { Preferences } from "../account/Preferences";
import { Security } from "../account/Security";
import { Session } from "../account/Session";
import { Referral } from "../account/Referral";
import { Rewards } from "../account/Rewards";

export const accountRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/account",
  component: AccountLayout,
});

export const accountIndexRoute = createRoute({
  getParentRoute: () => accountRoute,
  path: "/",
  beforeLoad: () => {
    throw redirect({ to: "/account/overview" });
  },
});

export const accountOverviewRoute = createRoute({
  getParentRoute: () => accountRoute,
  path: "/overview",
  component: Overview,
});

export const accountPortfolioRoute = createRoute({
  getParentRoute: () => accountRoute,
  path: "/portfolio",
  component: Portfolio,
});

export const accountPreferencesRoute = createRoute({
  getParentRoute: () => accountRoute,
  path: "/preferences",
  component: Preferences,
});

export const accountSecurityRoute = createRoute({
  getParentRoute: () => accountRoute,
  path: "/security",
  component: Security,
});

export const accountSessionRoute = createRoute({
  getParentRoute: () => accountRoute,
  path: "/session",
  component: Session,
});

export const accountReferralRoute = createRoute({
  getParentRoute: () => accountRoute,
  path: "/referral",
  component: Referral,
});

export const accountRewardsRoute = createRoute({
  getParentRoute: () => accountRoute,
  path: "/rewards",
  component: Rewards,
});
