import { createLazyFileRoute } from "@tanstack/react-router";

import { MobileTitle } from "~/components/foundation/MobileTitle";
import { DisplaySection } from "~/components/settings/DisplaySection";
import { KeyManagementSection } from "~/components/settings/KeyManagementSection";
import { SessionSection } from "~/components/settings/SessionSection";

import { m } from "~/paraglide/messages";

export const Route = createLazyFileRoute("/(app)/_app/settings")({
  component: SettingsApplet,
});

function SettingsApplet() {
  return (
    <div className="w-full md:max-w-[50rem] mx-auto flex flex-col gap-5 p-4 pt-6 mb-16">
      <MobileTitle title={m["settings.title"]()} />
      <SessionSection>
        <SessionSection.Username />
        <SessionSection.ConnectMobile />
        <SessionSection.RemainingTime />
        <SessionSection.Network />
      </SessionSection>
      <DisplaySection>
        <DisplaySection.Language />
        <DisplaySection.FormatNumber />
      </DisplaySection>
      <KeyManagementSection />
    </div>
  );
}
