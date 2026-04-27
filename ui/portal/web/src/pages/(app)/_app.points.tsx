import { createFileRoute } from "@tanstack/react-router";
import { z } from "zod";

import { PointsCampaign } from "~/components/points/PointsCampaign";

export const Route = createFileRoute("/(app)/_app/points")({
  component: RouteComponent,
  validateSearch: z.object({
    tab: z.enum(["profile", "rewards", "leaderboard"]).optional().default("profile"),
  }),
});

function RouteComponent() {
  const { tab } = Route.useSearch();
  const navigate = Route.useNavigate();

  const handleTabChange = (newTab: "profile" | "rewards" | "leaderboard") => {
    navigate({ search: { tab: newTab }, replace: true });
  };

  return (
    <PointsCampaign activeTab={tab} onTabChange={handleTabChange}>
      <PointsCampaign.Header />
      <PointsCampaign.Tabs />
    </PointsCampaign>
  );
}
