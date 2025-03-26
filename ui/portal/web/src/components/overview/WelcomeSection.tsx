import { Button } from "@left-curve/applets-kit";
import { useAccount, useBalances } from "@left-curve/store";
import { Link } from "@tanstack/react-router";
import type React from "react";
import { useApp } from "~/hooks/useApp";
import { m } from "~/paraglide/messages";
import { ButtonLink } from "../foundation/ButtonLink";
import { AssetsSection } from "./AssetsSection";
import { SwippeableAccountCard } from "./SwippeableAccountCard";

interface Props {
  cardMobileVisible: number;
  setCardMobileVisible: (value: number) => void;
}

export const WelcomeSection: React.FC<Props> = ({ cardMobileVisible, setCardMobileVisible }) => {
  const { account, isConnected } = useAccount();
  const { setSidebarVisibility } = useApp();
  const { data: balances = {} } = useBalances({ address: account?.address });

  if (!isConnected) {
    return (
      <div className="rounded-3xl relative shadow-card-shadow flex gap-4 w-full p-4 items-center flex-col lg:flex-row justify-end overflow-hidden min-h-[20rem] lg:min-h-[14.5rem] bg-[linear-gradient(236.46deg,_#FFF9F0_21.76%,_#E7D1B9_77.58%)]">
        <picture className="absolute left-0 lg:left-4 top-[-1rem] lg:top-auto max-h-44 lg:max-h-80 right-0 mx-auto lg:right-auto flex items-center justify-center">
          <source media="(min-width:1024px)" srcSet="/images/characters/group.svg" />
          <img rel="preload" src="/images/characters/group-mobile.svg" alt="group" />
        </picture>

        <div className=" lg:pr-[4.75rem]">
          <div className="flex flex-col gap-4 items-center max-w-[19.5rem] text-center">
            <p className="text-rice-800 exposure-h3-italic lg:exposure-h2-italic lg:!leading-normal">
              {m["common.motto"]()}
            </p>
            <div className="flex items-center justify-center gap-4 w-full lg:px-6">
              <Button as={Link} fullWidth to="/login">
                {m["common.signin"]()}
              </Button>

              <Button
                as={Link}
                fullWidth
                variant="secondary"
                to="/signup"
                className="hidden lg:block"
              >
                {m["common.signup"]()}
              </Button>
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="rounded-3xl bg-rice-50 shadow-card-shadow flex flex-col lg:flex-row gap-4 w-full p-4 items-center lg:items-start">
      <SwippeableAccountCard
        cardVisible={cardMobileVisible}
        setCardVisible={setCardMobileVisible}
      />

      <div className="w-full flex flex-col lg:gap-4 items-center">
        <div className="hidden lg:flex w-full">
          <AssetsSection
            balances={balances}
            showAllAssets={isConnected ? () => setSidebarVisibility(true) : undefined}
          />
        </div>

        {isConnected ? (
          <div className="lg:self-end flex gap-4 items-center justify-center w-full lg:max-w-[256px]">
            <ButtonLink fullWidth size="md" to="/send-and-receive" search={{ action: "receive" }}>
              {m["common.fund"]()}
            </ButtonLink>
            <ButtonLink
              fullWidth
              variant="secondary"
              size="md"
              to="/send-and-receive"
              search={{ action: "send" }}
            >
              {m["common.send"]()}
            </ButtonLink>
          </div>
        ) : null}
      </div>
    </div>
  );
};
