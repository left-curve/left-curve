import { createLazyFileRoute, useNavigate } from "@tanstack/react-router";

import { m } from "~/paraglide/messages";

import { SimpleSwap } from "~/components/dex/SimpleSwap";
import { MobileTitle } from "~/components/foundation/MobileTitle";

export const Route = createLazyFileRoute("/(app)/_app/swap")({
  component: SwapApplet,
});

function SwapApplet() {
  const navigate = useNavigate();
  const { from, to } = Route.useSearch();

  const onChangePair = (pair: { from: string; to: string }) => {
    navigate({ to: ".", search: pair, replace: true });
  };

  return (
    <div className="w-full md:max-w-[25rem] mx-auto flex flex-col p-4 pt-6 gap-4 min-h-[100svh] md:min-h-fit">
      <MobileTitle title={m["dex.convert.title"]()} />
      <SimpleSwap pair={{ from, to }} onChangePair={onChangePair}>
        <SimpleSwap.Header />
        <SimpleSwap.Form />
        <SimpleSwap.Details />
        <SimpleSwap.Trigger />
      </SimpleSwap>
    </div>
  );
}
