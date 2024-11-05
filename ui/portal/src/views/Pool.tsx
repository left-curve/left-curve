import { useState } from "react";
import { useSearchParams } from "react-router-dom";

import { PoolManagment, PoolSelector, Tab, Tabs } from "@dango/shared";

const actions = ["deposit", "withdraw"];

const PoolView: React.FC = () => {
  const [searchParams, setSearchParam] = useSearchParams();
  const action = searchParams.get("action") || "deposit";
  const poolId = searchParams.get("id");

  const [showPoolSelector, setShowPoolSelector] = useState(false);
  const [activePoolId, setActivePoolId] = useState<number>(Number.parseInt(poolId || "1"));
  const [activeAction, setActiveAction] = useState<string>(
    actions.includes(action) ? action : "deposit",
  );

  return (
    <div className="min-h-full w-full flex-1 flex items-center justify-center z-10 relative p-4">
      <div className="flex flex-col gap-8 w-full items-center justify-center max-w-[38.5rem]">
        <div className="w-full items-center justify-end flex">
          <Tabs
            key="dex-view-actions"
            defaultSelectedKey={activeAction}
            onSelectionChange={(key) => {
              const actionKey = key.toString();
              setSearchParam({ action: actionKey });
              setActiveAction(actionKey);
            }}
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
          <PoolSelector
            onPoolSelection={(id) => [
              setActivePoolId(id),
              setShowPoolSelector(false),
              setSearchParam({ id: id.toString() }),
            ]}
          />
        ) : (
          <PoolManagment
            poolId={activePoolId}
            action={activeAction}
            onRequestPoolSelection={() => setShowPoolSelector(true)}
          />
        )}
      </div>
    </div>
  );
};

export default PoolView;
