import { createFileRoute } from "@tanstack/react-router";

import { NotFound } from "~/components/foundation/NotFound";
import { ReferralCampaign } from "~/components/referral/ReferralCampaign";
import { isFeatureEnabled } from "~/featureFlags";

export const Route = createFileRoute("/(app)/_app/referral")({
  component: RouteComponent,
});

function RouteComponent() {
  if (!isFeatureEnabled("referral")) return <NotFound />;

  return (
    <ReferralCampaign>
      <ReferralCampaign.Header />
      <ReferralCampaign.Content />
    </ReferralCampaign>
  );
}
