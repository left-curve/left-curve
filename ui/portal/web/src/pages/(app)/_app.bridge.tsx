import { createFileRoute } from "@tanstack/react-router";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { MobileTitle } from "~/components/foundation/MobileTitle";
import { useState } from "react";
import { Bridge } from "~/components/bridge/Bridge";

export const Route = createFileRoute("/(app)/_app/bridge")({
  component: BridgeApplet,
});

function BridgeApplet() {
  const [action, setAction] = useState("deposit");
  const [network, setNetwork] = useState();

  return (
    <div className="w-full md:max-w-[27rem] mx-auto flex flex-col p-4 pt-6 gap-4 min-h-[100svh] md:min-h-fit">
      <MobileTitle title={m["bridge.title"]()} />
      <div className="w-full flex flex-col gap-4  md:pt-28 items-center justify-start ">
        <Bridge>
          <Bridge.Deposit />
          <Bridge.Withdraw />
        </Bridge>
      </div>
    </div>
  );
}
