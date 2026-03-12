import { createLazyFileRoute } from "@tanstack/react-router";

import { MobileTitle } from "~/components/foundation/MobileTitle";
import { DisplaySection } from "~/components/settings/DisplaySection";
import { KeyManagementSection } from "~/components/settings/KeyManagementSection";
import { SessionSection } from "~/components/settings/SessionSection";

import { m } from "@left-curve/foundation/paraglide/messages.js";

export const Route = createLazyFileRoute("/(app)/_app/settings")({
  component: SettingsApplet,
});

function SettingsApplet() {
  return (
    <div className="w-full md:max-w-[50rem] mx-auto flex flex-col gap-5 p-4 pt-6 mb-16">
      <MobileTitle title={m["settings.title"]()} />
      <div className="flex flex-col gap-3">
        <h3 className="diatype-lg-bold text-ink-primary-900">{m["settings.session.title"]()}</h3>
        <SessionSection>
          <SessionSection.Username />
          <SessionSection.UserStatus />
          <SessionSection.ConnectMobile />
          <SessionSection.RemainingTime />
          <SessionSection.Network />
        </SessionSection>
      </div>
      <div className="flex flex-col gap-3">
        <h3 className="diatype-lg-bold text-ink-primary-900">{m["settings.display"]()}</h3>
        <DisplaySection>
          <DisplaySection.Language />
          <DisplaySection.FormatNumber />
          <DisplaySection.DateFormat />
          <DisplaySection.TimeFormat />
          <DisplaySection.TimeZone />
          <DisplaySection.ChartEngine />
          <DisplaySection.Theme />
        </DisplaySection>
      </div>
      <div className="flex flex-col gap-3">
        <h3 className="diatype-lg-bold text-ink-primary-900">{m["settings.keyManagement.title"]()}</h3>
        <KeyManagementSection />
      </div>
    </div>
  );
}
