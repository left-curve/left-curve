import ReactFullpage from "@fullpage/react-fullpage";
import { createFileRoute } from "@tanstack/react-router";
import { m } from "~/paraglide/messages";

import { IconChevronDown, useMediaQuery, useTheme } from "@left-curve/applets-kit";
import { AppletsSection } from "~/components/overview/AppletsSection";
import { SearchMenu } from "~/components/foundation/SearchMenu";

export function getFullpageLicenseKey() {
  if (!process.env.NEXT_PUBLIC_FULLPAGE_KEY) return "FALLBACK_KEY";
  return new TextDecoder("utf-8", { fatal: true }).decode(
    Uint8Array.from(atob(process.env.NEXT_PUBLIC_FULLPAGE_KEY), (c) => c.charCodeAt(0)),
  );
}

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
    <div className="w-full mx-auto flex flex-col gap-6 pt-4 lg:pt-0 pb-16 flex-1">
      <ReactFullpage
        licenseKey={getFullpageLicenseKey()}
        scrollingSpeed={1000}
        credits={{ enabled: false }}
        render={({ fullpageApi }) => {
          return (
            <ReactFullpage.Wrapper>
              <div className="section min-h-svh flex items-center justify-center relative w-full">
                <div className="lg:max-w-3xl mx-auto p-4 w-full flex flex-col gap-6 relative flex-1 items-center justify-between lg:items-center lg:justify-center lg:gap-16 lg:pb-60">
                  <img
                    src={`/images/dango${theme === "dark" ? "-dark" : ""}.svg`}
                    alt="Dango"
                    className="max-w-[10rem] lg:max-w-[13rem]"
                  />
                  {isLg && (
                    <div className="relative w-full h-11">
                      {" "}
                      <SearchMenu />
                    </div>
                  )}
                  <AppletsSection />
                </div>
                <div
                  className="absolute bottom-[6rem] lg:bottom-[10rem] left-1/2 -translate-x-1/2 cursor-pointer diatype-m-medium"
                  onClick={() => fullpageApi.moveSectionDown()}
                >
                  <div className="animate-levitate flex items-center justify-center flex-col">
                    <p>{m["welcome.scroll"]()}</p>
                    <IconChevronDown className="w-6 h-6" />
                  </div>
                </div>
              </div>
              {/* {isSearchBarVisible ? null : (
                <>
                  <section className="section w-full min-h-svh flex items-center justify-center">
                    Section 1
                  </section>
                  <section className="section w-full min-h-svh flex items-center justify-center">
                    Section 2
                  </section>
                </>
              )} */}
            </ReactFullpage.Wrapper>
          );
        }}
      />
    </div>
  );
}
