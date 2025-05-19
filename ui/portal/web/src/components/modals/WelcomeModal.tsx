import { useConfig } from "@left-curve/store";

import { Button, twMerge } from "@left-curve/applets-kit";

import { m } from "~/paraglide/messages";

import type React from "react";
import { useApp } from "~/hooks/useApp";

export const WelcomeModal: React.FC = () => {
  const { chain } = useConfig();
  const { settings, changeSettings } = useApp();

  const { showWelcome } = settings;

  if (!showWelcome || chain.name !== "testnet") return null;

  return (
    <div
      className={twMerge(
        "w-screen h-screen bg-gray-900/50 fixed top-0 left-0 z-[51] flex items-center justify-center p-4 overflow-auto scrollbar-none py-32",
      )}
    >
      <div className="w-full flex flex-col items-center justify-start bg-white-100 rounded-xl border border-gray-100 max-w-96 md:max-w-2xl">
        <div className="flex flex-col gap-4 p-4 border-b border-b-gray-100">
          <div className="w-12 h-12 rounded-full flex items-center justify-center">
            <img
              src="/favicon.svg"
              alt="dango logo"
              className={"h-11 order-1 cursor-pointer flex rounded-full shadow-btn-shadow-gradient"}
            />
          </div>
          <p className="h4-bold">{m["common.testnet.title"]()}</p>
          <div className="flex flex-col diatype-m-medium text-gray-500 gap-2">
            <p>Thank you for trying out Dango's first ever testnet!</p>

            <p>
              The purpose of testnet-1 is{" "}
              <span className="diatype-m-bold">for testing the basic infrastructure</span> of Dango
              chain, including consensus, smart contract engine, and indexer. As such, user-facing
              features are very limited.
              <br /> The coming testnet-2 (expected in June/July) will add our flagship product, the{" "}
              <span className="diatype-m-bold">Dango DEX</span>. Then, testnet-3 (August/September)
              will add margin trading.
            </p>

            <p>
              For now, you can <span className="diatype-m-bold">sign up, open subaccounts</span>,
              and <span className="diatype-m-bold">transfer tokens</span>, that's it. Upon your
              first login, you will receive some fake tokens (fake BTC, fake ETH, ...) to play with.
            </p>

            <p>
              Make sure to complete the
              <Button
                as="a"
                className="p-0"
                variant="link"
                href="https://app.galxe.com/quest/dango/"
                target="_blank"
                rel="noreferrer"
              >
                Galxe quests
              </Button>
              and mint your limited-time NFT!
            </p>

            <p>g🍡</p>

            <Button
              as="a"
              href="https://x.com/larry0x"
              target="_blank"
              rel="noreferrer"
              variant="link"
              className="p-0"
            >
              @larry0x
            </Button>
          </div>
        </div>
        <div className="p-4 w-full">
          <Button
            variant="secondary"
            fullWidth
            onClick={() => changeSettings({ showWelcome: false })}
          >
            {m["common.dismiss"]()}
          </Button>
        </div>
      </div>
    </div>
  );
};
