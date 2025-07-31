import { createFileRoute } from "@tanstack/react-router";
import { m } from "~/paraglide/messages";

import { useMediaQuery, useTheme } from "@left-curve/applets-kit";
import { AppletsSection } from "~/components/overview/AppletsSection";
import { SearchMenu } from "~/components/foundation/SearchMenu";

export const Route = createFileRoute("/(app)/_app/")({
  head: () => ({
    meta: [{ title: `Dango | ${m["common.overview"]()}` }],
  }),
  component: OverviewComponent,
});

function OverviewComponent() {
  const { isLg } = useMediaQuery();
  const { theme } = useTheme();
  return (
    <div className="w-full lg:max-w-3xl mx-auto flex flex-col gap-6 p-4 pt-6 pb-16 flex-1">
      <div className="w-full flex flex-col gap-6 min-h-full lg:min-h-fit relative flex-1 items-center justify-between lg:items-center lg:justify-center lg:gap-16 lg:pb-60">
        <img
            src={`/images/dango${theme === "dark" ? "-dark" : ""}.svg`}
            alt="Dango"
            className="max-w-[10rem] lg:max-w-[13rem]"
            />
            {isLg && <div className="relative w-full h-11"> <SearchMenu /></div>}
        <AppletsSection />
      </div>
    </div>
  );
}
