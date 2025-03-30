import { Button } from "@left-curve/applets-kit";

import { m } from "~/paraglide/messages";

import { useStorage } from "@left-curve/store";
import type React from "react";

export const WelcomeModal: React.FC = () => {
  const [showWelcome, setShowWelcome] = useStorage("showWelcome", {
    initialValue: true,
  });

  if (!showWelcome) return null;

  return (
    <div className="w-screen h-screen bg-gray-900/50 fixed top-0 left-0 z-[51] flex items-center justify-center p-4">
      <div className="w-full flex flex-col items-center justify-start bg-white-100 rounded-xl border border-gray-100 max-w-96 md:max-w-xl">
        <div className="flex flex-col gap-4 p-4 border-b border-b-gray-100">
          <div className="w-12 h-12 rounded-full flex items-center justify-center">
            <img
              src="/favicon.svg"
              alt="dango logo"
              className={"h-11 order-1 cursor-pointer flex rounded-full shadow-btn-shadow-gradient"}
            />
          </div>
          <p className="h4-bold">{m["common.testnet.title"]()}</p>
          <p className="diatype-m-medium text-gray-500">{m["common.testnet.description"]()}</p>
        </div>
        <div className="p-4 w-full">
          <Button variant="secondary" fullWidth onClick={() => setShowWelcome(false)}>
            {m["common.dismiss"]()}
          </Button>
        </div>
      </div>
    </div>
  );
};
