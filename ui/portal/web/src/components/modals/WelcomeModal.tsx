import { useConfig } from "@left-curve/store";

import { Button, twMerge } from "@left-curve/applets-kit";

import { m } from "~/paraglide/messages";

import type React from "react";
import { useApp } from "~/hooks/useApp";

export const WelcomeModal: React.FC = () => {
  const { chain } = useConfig();
  const { settings, changeSettings } = useApp();

  const { showWelcome } = settings;

  if (!showWelcome || chain.name !== "Testnet") return null;

  return (
    <div
      className={twMerge(
        "w-screen h-screen bg-gray-900/50 fixed top-0 left-0 z-[51] flex items-center justify-center p-4 overflow-auto scrollbar-none py-32",
      )}
    >
      <div className="w-full flex flex-col items-center justify-start bg-surface-primary-rice rounded-xl border border-secondary-gray max-w-2xl">
        <div className="flex flex-col gap-4 p-4 border-b border-b-secondary-gray">
          <div className="w-12 h-12 rounded-full flex items-center justify-center">
            <img
              src="/favicon.svg"
              alt="dango logo"
              className={"h-11 order-1 cursor-pointer flex rounded-full shadow-btn-shadow-gradient"}
            />
          </div>
          <p className="h4-bold">{m["common.testnet.title"]()}</p>
          <div className="flex flex-col diatype-m-medium text-tertiary-500 gap-4">
            <p>Hi there,</p>
            <p>
              Thank you for participating in <span className="font-bold">testnet-2</span>!
            </p>

            <p>
              The main feature of this testnet is our{" "}
              <span className="font-bold">onchain order book exchange</span>
              <Button
                as="a"
                href="https://x.com/larry0x/status/1947685791167353284%7D"
                target="_blank"
                rel="noreferrer"
                variant="link"
                className="p-0 h-fit"
              >
                learn more
              </Button>
              . Click the "Trade" icon on the homepage, or enter "trade" in the search bar to
              access.
            </p>

            <p>
              There is a quest on the
              <Button
                as="a"
                href="https://app.galxe.com/quest/dango/GCMTJtfErm%7D"
                target="_blank"
                rel="noreferrer"
                variant="link"
                className="p-0 h-fit"
              >
                Galxe
              </Button>
              platform. The tasks are related to trading, e.g. trading a certain volume, in various
              pairs, with various order types... don't forget to{" "}
              <span className="font-bold">complete the quest</span> and{" "}
              <span>claim the limited time OAT</span>.
            </p>

            <p>
              Please be aware this is <span className="font-bold">pre-alpha software</span>, which
              doesn't represent final mainnet experience. Expect missing features, bugs, and
              hiccups. Don't hesitate to send us feedback on{" "}
              <a href="https://discord.gg/4uB9UDzYhz%7D">Discord</a>
            </p>

            <p>Have fun and g🍡</p>

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
