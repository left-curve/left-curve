import { createLazyFileRoute } from "@tanstack/react-router";
import { MsgBuilder } from "~/components/devtools/MsgBuilder";
import { MobileTitle } from "~/components/foundation/MobileTitle";

import { m } from "@left-curve/foundation/paraglide/messages.js";

export const Route = createLazyFileRoute("/(app)/_app/devtool")({
  component: DevtoolApplet,
});

function DevtoolApplet() {
  return (
    <div className="w-full flex flex-col items-center">
      <MobileTitle title={m["devtools.title"]()} className="p-4 pb-0" />
      <MsgBuilder>
        <MsgBuilder.QueryMsg />
        <MsgBuilder.ExecuteMsg />
      </MsgBuilder>
    </div>
  );
}
