import { useState } from "react";
import { useSearchParams } from "react-router-dom";

import { Tab, Tabs } from "@dango/shared";
import { SendContainer } from "~/components/SendContainer";
import { TransferContainer } from "~/components/TransferContainer";

const Transfer: React.FC = () => {
  const [searchParams] = useSearchParams();
  const [activeAction, setActiveAction] = useState<string>(searchParams.get("action") || "send");

  return (
    <div className="min-h-full w-full flex-1 flex items-center justify-center z-10 relative p-4">
      <div className="flex flex-col gap-8 w-full items-center justify-center max-w-[38.5rem]">
        <div className="w-full items-center justify-end flex">
          <div>
            <Tabs
              key="transfer-actions"
              defaultSelectedKey={activeAction}
              onSelectionChange={(key) => setActiveAction(key.toString())}
              classNames={{
                tabsWrapper: "p-1 bg-surface-green-300 text-typography-green-300 rounded-2xl gap-0",
              }}
            >
              {["send", "transfer"].map((action) => (
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
        </div>
        {activeAction === "send" ? <SendContainer /> : null}
        {activeAction === "transfer" ? <TransferContainer /> : null}
      </div>
    </div>
  );
};

export default Transfer;
