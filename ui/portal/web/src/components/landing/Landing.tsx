import { useEffect } from "react";
import { useApp, useMediaQuery, useTheme } from "@left-curve/applets-kit";

import {
  Button,
  createContext,
  IconChevronDown,
  IconDiscord,
  IconMirror,
  IconTwitter,
} from "@left-curve/applets-kit";
import { SearchMenu } from "../foundation/SearchMenu";
import { AppletsSection } from "../overview/AppletsSection";

import { decodeBase64, decodeUtf8 } from "@left-curve/dango/encoding";
import ReactFullpage from "@fullpage/react-fullpage";
import { m } from "~/paraglide/messages";
import { format } from "date-fns";

import type { fullpageApi } from "@fullpage/react-fullpage";
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
        beforeLeave={(_, _destination) => setQuestBannerVisibility(false)}
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
  const { isSidebarVisible } = useApp();
  const { isLg } = useMediaQuery();
  const { theme } = useTheme();
  const { fullpageApi } = useLanding();

  useEffect(() => {
    if (!fullpageApi) return;
    fullpageApi.setAllowScrolling(!isSidebarVisible);
  }, [fullpageApi, isSidebarVisible]);

  return (
    <div className="section min-h-svh flex items-center justify-center relative w-full">
      <div className="min-h-[calc(100svh-5svh)] mx-auto pb-[20svh] p-4 lg:p-0 w-full flex flex-col gap-6 relative flex-1 items-center justify-between lg:items-center lg:justify-center lg:gap-16 lg:pb-60">
        <img
          src={`/images/dango${theme === "dark" ? "-dark" : ""}.svg`}
          alt="Dango"
          className="max-w-[10rem] lg:max-w-[13rem]"
        />
        {isLg && (
          <div className="relative w-full h-11 z-40 max-w-[38rem]">
            <SearchMenu />
          </div>
        )}
        <div className="flex w-full max-w-[38rem]">
          <AppletsSection />
        </div>
      </div>
      <div
        className="absolute bottom-[12svh] lg:bottom-[3rem] left-1/2 -translate-x-1/2 cursor-pointer diatype-m-medium"
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
    <section className="section w-full min-h-svh flex items-center justify-center bg-surface-primary-rice bg-[linear-gradient(212.63deg,_rgba(255,_229,_190,_0.4)_19.52%,_#FFDEAE_94.1%)] dark:bg-[linear-gradient(212.63deg,_#42403D_19.52%,_#807668_94.1%)]">
      <div className="max-w-[76rem] w-full mx-auto flex flex-col lg:flex-row items-center lg:justify-between p-4 pr-0">
        <div className="flex flex-col gap-2 max-w-[40rem]">
          <h1 className="display-heading-l md:display-heading-2xl text-rice-600 dark:text-secondary-rice">
            {m["welcome.trade"]()}
          </h1>
          <p className="diatype-m-regular md:h1-medium text-rice-600 dark:text-secondary-rice">
            {m["welcome.tradeText"]()}
          </p>
          <p className="diatype-m-regular md:h1-medium text-rice-600 dark:text-secondary-rice">
            {m["welcome.tradeText2"]()}
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
    <section className="section w-full min-h-svh flex items-center justify-center bg-surface-primary-rice bg-[linear-gradient(212.63deg,_rgba(255,_221,_223,_0.4)_19.52%,_#FFD0D3_94.1%)] dark:bg-[linear-gradient(212.63deg,_#383634_19.52%,_#6A6361_94.1%)]">
      <div className="max-w-[76rem] w-full mx-auto flex flex-col lg:flex-row items-center lg:justify-between p-4 pr-0">
        <div className="flex flex-col gap-2 max-w-[40rem]">
          <h1 className="display-heading-l md:display-heading-2xl text-tertiary-red">
            {m["welcome.leverageUp"]()}
          </h1>
          <p className="diatype-m-regular md:h1-medium text-tertiary-red">
            {m["welcome.leverageUpText"]()}
          </p>
          <p className="diatype-m-regular md:h1-medium text-tertiary-red">
            {m["welcome.leverageUpText2"]()}
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
    <section className="section w-full min-h-svh flex items-center justify-center bg-surface-primary-rice bg-[linear-gradient(212.63deg,_rgba(239,_240,_173,_0.4)_19.52%,_#EFF0AD_94.1%)] dark:bg-[linear-gradient(212.63deg,_#373634_19.52%,_#666654_94.1%)]">
      <div className="max-w-[76rem] w-full mx-auto flex flex-col lg:flex-row items-center lg:justify-between p-4 pr-0">
        <div className="flex flex-col gap-2 max-w-[40rem]">
          <h1 className="display-heading-l md:display-heading-2xl text-green-bean-800 dark:text-foreground-primary-green">
            {m["welcome.earn"]()}
          </h1>
          <p className="diatype-m-regular md:h1-medium text-green-bean-800 dark:text-foreground-primary-green">
            {m["welcome.earnText"]()}
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

const SectionCommunity: React.FC = () => {
  const { isSearchBarVisible } = useApp();

  if (isSearchBarVisible) return null;

  return (
    <section className="section w-full min-h-svh flex items-center justify-center lg:justify-end bg-surface-primary-rice bg-[linear-gradient(6.97deg,_#D0CFEB_11.63%,_#F6F6FB_88.19%)] dark:bg-[linear-gradient(6.97deg,_#6E6D77_11.63%,_#373634_88.19%)]">
      <div className="max-w-[76rem] w-full mx-auto flex flex-col p-4 pb-[10svh] lg:pb-0 min-h-[calc(100svh)] lg:justify-center pt-[76px]">
        <div className="w-full flex flex-col lg:flex-row items-center lg:justify-between gap-4 flex-1">
          <img
            src="/images/characters/friends.svg"
            alt="rabbits"
            className="w-full transition-all max-w-[317px] md:max-w-[435px] lg:max-h-[535px]"
          />
          <div className="flex flex-col max-w-[33rem] items-center justify-center text-center gap-6 lg:gap-8 z-30">
            <h2 className="display-heading-m md:display-heading-2xl">
              {m["welcome.join"]()}
              <span className="text-red-bean-400">{m["welcome.community"]()}</span>{" "}
              {m["welcome.of"]()}{" "}
              <span className="text-red-bean-400">{m["welcome.dangbros"]()}</span>
            </h2>
            <div className="flex gap-4">
              <Button
                as="a"
                href="https://x.com/dango"
                target="_blank"
                rel="noopener noreferrer"
                className="gap-0"
              >
                <IconTwitter className="w-6 h-6" />
                <span>Twitter</span>
              </Button>
              <Button
                as="a"
                href="https://discord.gg/4uB9UDzYhz"
                target="_blank"
                rel="noopener noreferrer"
                className="gap-0"
              >
                <IconDiscord className="w-5 h-5" />
                <span className="pl-[6px]">Discord</span>
              </Button>
              <Button
                as="a"
                href="https://mirror.xyz/0x8E4AA2B6F137D2eD6Ba3E3Bb8E64240D46035DE6"
                target="_blank"
                rel="noopener noreferrer"
                className="gap-0"
              >
                <IconMirror className="w-5 h-5" />
                <span className="pl-[6px]">Mirror</span>
              </Button>
            </div>
          </div>
        </div>
        <div className="w-full border-t border-t-border-tertiary-blue items-center justify-between flex py-6 diatype-m-medium flex-col lg:flex-row gap-1">
          <p>© 2024-{format(new Date(), "yy")} Left Curve Software</p>
          <div className="flex gap-10 lg:gap-4 diatype-m-medium">
            <a
              href="/documents/Dango%20-%20Terms%20of%20Use.pdf"
              target="_blank"
              rel="noopener noreferrer"
              className="hover:underline text-tertiary-500"
            >
              {m["welcome.termsOfUse"]()}
            </a>

            <a
              href="/documents/Dango%20-%20Privacy%20Policy.pdf"
              target="_blank"
              rel="noopener noreferrer"
              className="hover:underline text-tertiary-500"
            >
              {m["welcome.privacyPolicy"]()}
            </a>
          </div>
        </div>
      </div>
    </section>
  );
};

export const Landing = Object.assign(LandingContainer, {
  Header,
  SectionRice,
  SectionRed,
  SectionGreen,
  SectionCommunity,
});
