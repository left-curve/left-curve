import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { z } from "zod";

import { ReferralCampaign } from "~/components/referral/ReferralCampaign";

export const Route = createFileRoute("/(app)/_app/referral")({
  component: RouteComponent,
  validateSearch: z.object({
    tab: z.enum(["affiliate", "trader"]).optional().default("affiliate"),
  }),
});

function RouteComponent() {
  const { tab } = Route.useSearch();
  const navigate = useNavigate();

  const handleTabChange = (newTab: "affiliate" | "trader") => {
    navigate({ search: { tab: newTab }, replace: true });
  };

  return (
    <ReferralCampaign activeTab={tab} onTabChange={handleTabChange}>
      <ReferralCampaign.Header />
      <ReferralCampaign.Content />
    </ReferralCampaign>
  );
}
