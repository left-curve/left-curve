import { createFileRoute } from "@tanstack/react-router";

import { PointsCampaign } from "~/components/points/PointsCampaign";

export const Route = createFileRoute("/(app)/_app/points")({
  component: RouteComponent,
});

function RouteComponent() {
  return (
    <PointsCampaign>
      <PointsCampaign.Header />
      <PointsCampaign.Tabs />
    </PointsCampaign>
  );
}
