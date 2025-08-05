import { createLazyFileRoute } from "@tanstack/react-router";

import { MsgBuilder } from "~/components/devtools/MsgBuilder";

export const Route = createLazyFileRoute("/(app)/_app/devtool")({
  component: DevtoolApplet,
});

function DevtoolApplet() {
  return <MsgBuilder />;
}
