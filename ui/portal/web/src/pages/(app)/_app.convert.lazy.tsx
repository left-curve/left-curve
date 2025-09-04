import { createLazyFileRoute, useNavigate } from "@tanstack/react-router";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { Convert } from "@left-curve/applets-convert";
import { MobileTitle } from "~/components/foundation/MobileTitle";
import { useApp } from "@left-curve/foundation";

export const Route = createLazyFileRoute("/(app)/_app/convert")({
  component: ConvertApplet,
});

function ConvertApplet() {
  const navigate = useNavigate();
  const app = useApp();
  const { from, to } = Route.useSearch();

  const onChangePair = (pair: { from: string; to: string }) => {
    navigate({ to: ".", search: pair, replace: true });
  };

  return (
    <div className="w-full md:max-w-[25rem] mx-auto flex flex-col p-4 pt-6 gap-4 min-h-[100svh] md:min-h-fit">
      <MobileTitle title={m["dex.convert.title"]()} />
      <Convert pair={{ from, to }} onChangePair={onChangePair} appState={app}>
        <Convert.Header />
        <Convert.Form />
        <Convert.Details />
        <Convert.Trigger />
      </Convert>
    </div>
  );
}
