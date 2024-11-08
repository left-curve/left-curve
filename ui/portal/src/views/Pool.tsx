import { useEffect, useState } from "react";

import { PoolManagment, PoolSelector, Tab, Tabs } from "@dango/shared";
import { parseAsInteger, useQueryState } from "nuqs";

const actions = ["deposit", "withdraw"];

const PoolView: React.FC = () => {
  const [poolId, setPoolId] = useQueryState("pool", parseAsInteger);
  const [action, setAction] = useQueryState("action");

  const [showPoolSelector, setShowPoolSelector] = useState(false);

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
          <PoolSelector onPoolSelected={() => setShowPoolSelector(false)} />
        ) : (
          <PoolManagment onRequestPoolSelection={() => setShowPoolSelector(true)} />
        )}
      </div>
    </div>
  );
};

export default PoolView;
