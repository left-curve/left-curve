import ReactFullpage, { type fullpageApi } from "@fullpage/react-fullpage";

import { createContext, IconChevronDown, useMediaQuery, useTheme } from "@left-curve/applets-kit";
import { m } from "~/paraglide/messages";
import { SearchMenu } from "../foundation/SearchMenu";
import { AppletsSection } from "../overview/AppletsSection";
import { useApp } from "~/hooks/useApp";
import { decodeBase64, decodeUtf8 } from "@left-curve/dango/encoding";

import type { PropsWithChildren } from "react";

type LandingProps = {
  fullpageApi: fullpageApi;
};

const [LandingProvider, useLanding] = createContext<LandingProps>({
  name: "LandingContext",
});

const LandingContainer: React.FC<PropsWithChildren> = ({ children }) => {
  const { setQuestBannerVisibility } = useApp();
  return (
    <div className="w-full mx-auto flex flex-col gap-6 pt-0 pb-16 flex-1">
      <ReactFullpage
        beforeLeave={(_, destination) => setQuestBannerVisibility(destination.isFirst)}
        licenseKey={decodeUtf8(decodeBase64(import.meta.env.PUBLIC_FP || "RkFMTEJBQ0tfS0VZCg=="))}
        scrollingSpeed={1000}
        credits={{ enabled: false }}
        render={({ fullpageApi }) => {
          return (
            <ReactFullpage.Wrapper>
              <LandingProvider value={{ fullpageApi }}>{children}</LandingProvider>
            </ReactFullpage.Wrapper>
          );
        }}
      />
    </div>
  );
};

const Header: React.FC = () => {
  const { isLg } = useMediaQuery();
  const { theme } = useTheme();
  const { fullpageApi } = useLanding();

  return (
    <div className="section min-h-svh flex items-center justify-center relative w-full">
      <div className="lg:max-w-3xl min-h-svh mx-auto pb-[15rem] p-4 w-full flex flex-col gap-6 relative flex-1 items-center justify-between lg:items-center lg:justify-center lg:gap-16 lg:pb-60">
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
        className="absolute bottom-[8rem] lg:bottom-[10rem] left-1/2 -translate-x-1/2 cursor-pointer diatype-m-medium"
        onClick={() => fullpageApi.moveSectionDown()}
      >
        <div className="animate-levitate flex items-center justify-center flex-col">
          <p>{m["welcome.scroll"]()}</p>
          <IconChevronDown className="w-6 h-6" />
        </div>
      </div>
    </div>
  );
};

const SectionRice: React.FC = () => {
  const { isSearchBarVisible } = useApp();
  if (isSearchBarVisible) return null;

  return (
    <section className="section w-full min-h-svh flex items-center justify-center bg-surface-primary-rice bg-[linear-gradient(212.63deg,_rgba(255,_229,_190,_0.4)_19.52%,_#FFDEAE_94.1%)] dark:bg-[linear-gradient(212.63deg,_#42403D_19.52%,_#807668_94.1%)] p-4 lg:p-0">
      <div className="max-w-[76rem] w-full mx-auto flex flex-col lg:flex-row items-center">
        <div className="flex flex-col gap-2 max-w-[40rem]">
          <h1 className="display-heading-l md:display-heading-2xl text-rice-600 dark:text-secondary-rice">
            Trade
          </h1>
          <p className="diatype-m-regular md:h1-medium text-rice-600 dark:text-secondary-rice">
            crypto assets, real world assets, and derivatives and Dango’s blazingly fast exchange.
            Enjoy deep liquidity, fast execution, and fair prices.
          </p>
        </div>
        <img
          src="/images/emojis/detailed/temple.svg"
          alt="dango-temple"
          className="w-[90%] lg:w-full max-w-[535px]"
        />
      </div>
    </section>
  );
};

const SectionRed: React.FC = () => {
  const { isSearchBarVisible } = useApp();
  if (isSearchBarVisible) return null;

  return (
    <section className="section w-full min-h-svh flex items-center justify-center bg-surface-primary-rice bg-[linear-gradient(212.63deg,_rgba(255,_221,_223,_0.4)_19.52%,_#FFD0D3_94.1%)] dark:bg-[linear-gradient(212.63deg,_#383634_19.52%,_#6A6361_94.1%)] p-4 lg:p-0">
      <div className="max-w-[76rem] w-full mx-auto flex flex-col lg:flex-row items-center">
        <div className="flex flex-col gap-2 max-w-[40rem]">
          <h1 className="display-heading-l md:display-heading-2xl text-tertiary-red">
            Leverage up
          </h1>
          <p className="diatype-m-regular md:h1-medium text-tertiary-red">
            with Dango’s unified trading account with low cost and high capital efficiency. Spot,
            perps, vaults; one account, under a unified margin system.
          </p>
        </div>
        <img
          src="/images/emojis/detailed/fisher.svg"
          alt="dango-fisher"
          className="w-[90%] lg:w-full max-w-[535px]"
        />
      </div>
    </section>
  );
};

const SectionGreen: React.FC = () => {
  const { isSearchBarVisible } = useApp();
  if (isSearchBarVisible) return null;

  return (
    <section className="section w-full min-h-svh flex items-center justify-center bg-surface-primary-rice bg-[linear-gradient(212.63deg,_rgba(239,_240,_173,_0.4)_19.52%,_#EFF0AD_94.1%)] dark:bg-[linear-gradient(212.63deg,_#373634_19.52%,_#666654_94.1%)] p-4 lg:p-0">
      <div className="max-w-[76rem] w-full mx-auto flex flex-col lg:flex-row items-center">
        <div className="flex flex-col gap-2 max-w-[40rem]">
          <h1 className="display-heading-l md:display-heading-2xl text-green-bean-800 dark:text-foreground-primary-green">
            Earn
          </h1>
          <p className="diatype-m-regular md:h1-medium text-green-bean-800 dark:text-foreground-primary-green">
            passive yields on your idle assets, by participating in Dango’s passive market making
            vaults. Make your money work for you!
          </p>
        </div>
        <img
          src="/images/emojis/detailed/pig.svg"
          alt="dango-pig"
          className="w-[90%] lg:w-full max-w-[535px]"
        />
      </div>
    </section>
  );
};

export const Landing = Object.assign(LandingContainer, {
  Header,
  SectionRice,
  SectionRed,
  SectionGreen,
});
