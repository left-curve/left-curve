import { createLazyRoute, useNavigate, useSearch } from "@tanstack/react-router";
import { useEffect, useState } from "react";

import { PoolManagment, PoolSelector, Tab, Tabs } from "@left-curve/portal-shared";

const actions = ["deposit", "withdraw"];

export const AmmRoute = createLazyRoute("/amm")({
  component: () => {
    const { poolId, action } = useSearch({ strict: false });
    const navigate = useNavigate({ from: "." });

    const [showPoolSelector, setShowPoolSelector] = useState(false);

    const setPoolId = (id: number) => navigate({ search: { poolId: id } });
    const setAction = (action: string) => navigate({ search: { action } });

    useEffect(() => {
      if (!action) setAction("deposit");
      if (!poolId) setPoolId(0);
    }, []);

    return (
      <div className="min-h-full w-full flex-1 flex items-center justify-center z-10 relative p-4">
        <div className="flex flex-col gap-8 w-full items-center justify-center max-w-[38.5rem]">
          <div className="w-full items-center justify-end flex">
            <Tabs
              key="dex-view-actions"
              defaultSelectedKey={action || ""}
              onSelectionChange={(key) => setAction(key.toString())}
              classNames={{
                container: "w-fit",
                tabsWrapper: "p-1 bg-surface-green-300 text-typography-green-300 rounded-2xl gap-0",
              }}
            >
              {actions.map((action) => (
                <Tab
                  key={action}
                  title={action}
                  classNames={{
                    container: "after:rounded-xl font-bold capitalize",
                    selected: "after:bg-surface-green-400 text-typography-green-400",
                  }}
                />
              ))}
            </Tabs>
          </div>
          {showPoolSelector ? (
            <PoolSelector onPoolSelected={(id) => [setPoolId(id), setShowPoolSelector(false)]} />
          ) : (
            <PoolManagment
              poolId={poolId}
              action={action}
              onRequestPoolSelection={() => setShowPoolSelector(true)}
            />
          )}
        </div>
      </div>
    );
  },
});
