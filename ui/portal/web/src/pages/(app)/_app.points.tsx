import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { z } from "zod";

import { NotFound } from "~/components/foundation/NotFound";
import { PointsCampaign } from "~/components/points/PointsCampaign";
import { isFeatureEnabled } from "~/featureFlags";

export const Route = createFileRoute("/(app)/_app/points")({
  component: RouteComponent,
  validateSearch: z.object({
    tab: z.enum(["profile", "rewards"]).optional().default("profile"),
  }),
});

function RouteComponent() {
  const { tab } = Route.useSearch();
  const navigate = useNavigate();

  if (!isFeatureEnabled("points")) return <NotFound />;

  const handleTabChange = (newTab: "profile" | "rewards") => {
    navigate({ search: { tab: newTab }, replace: true });
  };

  return (
    <PointsCampaign activeTab={tab} onTabChange={handleTabChange}>
      <PointsCampaign.Header />
      <PointsCampaign.Tabs />
    </PointsCampaign>
  );
}
