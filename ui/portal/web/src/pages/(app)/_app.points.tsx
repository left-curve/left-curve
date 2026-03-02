import { createFileRoute } from "@tanstack/react-router";

import { NotFound } from "~/components/foundation/NotFound";
import { PointsCampaign } from "~/components/points/PointsCampaign";
import { isFeatureEnabled } from "~/featureFlags";

export const Route = createFileRoute("/(app)/_app/points")({
  component: RouteComponent,
});

function RouteComponent() {
  if (!isFeatureEnabled("points")) return <NotFound />;

  return (
    <PointsCampaign>
      <PointsCampaign.Header />
      <PointsCampaign.Tabs />
    </PointsCampaign>
  );
}
