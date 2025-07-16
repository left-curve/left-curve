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
          <div className="flex flex-col diatype-m-medium text-tertiary-500 gap-2">
            <p>Hey guys,</p>
            <p>We have launched testnet-1.5!</p>

            <p>
              As the name suggests, this is{" "}
              <span className="font-bold">
                a small incremental update from the previous testnet-1
              </span>
              . We shipped{" "}
              <span className="font-bold">
                various bug fixes, missing features, and adjustments based on your feedback
              </span>
              . These include:
            </p>
            <ul className="list-disc pl-4">
              <li>Notification system now works</li>
              <li>"Forgot username?" feature</li>
              <li>A simple block explorer</li>
              <li>A preview for the simple token swap widget</li>
            </ul>

            <p>As before, these features are our alpha build, so expect bugs and hiccups.</p>
            <p>
              Reminder-- <span className="font-bold">in ~6 weeks, we expect to ship testnet-2</span>
              , which will come with our <span className="font-bold">Pro Trading interface</span>,
              with features experience on par with major CEXs. Galxe quests are also coming.
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
