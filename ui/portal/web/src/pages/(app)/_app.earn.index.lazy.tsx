import { createLazyFileRoute, useNavigate } from "@tanstack/react-router";
import { Earn } from "~/components/earn/Earn";

export const Route = createLazyFileRoute("/(app)/_app/earn/")({
  component: EarnApplet,
});

function EarnApplet() {
  const navigate = useNavigate();
  return (
    <div className="w-full md:max-w-[76rem] mx-auto flex flex-col pt-6 mb-16">
      <Earn
        navigate={({ baseSymbol, quoteSymbol }) =>
          navigate({
            to: "/earn/pool/$pairSymbols",
            params: { pairSymbols: `${baseSymbol}-${quoteSymbol}` },
          })
        }
      >
        <Earn.Header />
        <Earn.PoolsCards />
        <Earn.UserPoolsTable />
      </Earn>
    </div>
  );
}
