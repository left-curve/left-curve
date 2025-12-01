import { createLazyFileRoute, useNavigate } from "@tanstack/react-router";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { MobileTitle } from "~/components/foundation/MobileTitle";
import { Bridge } from "~/components/bridge/Bridge";

export const Route = createLazyFileRoute("/(app)/_app/bridge")({
  component: BridgeApplet,
});
function BridgeApplet() {
  const navigate = useNavigate();
  const { action } = Route.useSearch();

  const changeAction = (action: "deposit" | "withdraw") => {
    navigate({
      to: ".",
      search: { action },
    });
  };

  return (
    <div className="w-full md:max-w-[27rem] mx-auto flex flex-col p-4 pt-6 gap-4 min-h-[100svh] md:min-h-fit">
      <MobileTitle title={m["bridge.title"]()} />
      <div className="w-full flex flex-col gap-4  md:py-10 items-center justify-start ">
        <Bridge action={action} changeAction={changeAction}>
          <Bridge.Deposit />
          <Bridge.Withdraw />
        </Bridge>
      </div>
    </div>
  );
}
