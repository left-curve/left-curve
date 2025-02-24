import { motion } from "framer-motion";
import React, { Children, cloneElement, type PropsWithChildren } from "react";
import { twMerge } from "../../utils";

export interface TabsProps {
  onTabChange?: (tab: string) => void;
  defaultKey?: string;
  keys?: string[];
}

export const Tabs: React.FC<PropsWithChildren<TabsProps>> = ({
  onTabChange,
  children,
  keys,
  defaultKey,
}) => {
  const tabs = keys ? keys : Children.toArray(children);
  const [activeTab, setActiveTab] = React.useState(() => {
    if (defaultKey) return defaultKey;

    if (tabs.length > 0) {
      return typeof tabs[0] === "string" ? tabs[0] : (tabs[0] as React.ReactElement).props.title;
    }
    return "";
  });

  React.useEffect(() => {
    onTabChange?.(activeTab);
  }, [activeTab]);

  return (
    <motion.div className="flex text-base relative  items-center w-fit bg-green-bean-200 p-1 rounded-md">
      {tabs.map((e, i) => {
        const isKey = typeof e === "string";
        const elemKey = isKey ? e : (e as React.ReactElement).props.title;
        const isActive = elemKey === activeTab;

        return (
          <motion.button
            className="relative transition-all flex items-center justify-center py-2 px-4 cursor-pointer"
            key={`navLink-${e}`}
            onClick={() => setActiveTab(elemKey)}
          >
            {isKey ? (
              <Tab key={elemKey} title={elemKey} isActive={isActive} />
            ) : (
              cloneElement(e as React.ReactElement, { isActive })
            )}
            {isActive ? (
              <motion.div
                className="w-full h-full rounded-[10px] bg-green-bean-50 absolute bottom-0 left-0 [box-shadow:0px_4px_6px_2px_#1919191F]"
                layoutId="active"
              />
            ) : null}
          </motion.button>
        );
      })}
    </motion.div>
  );
};

export interface TabProps {
  title: string;
  isActive?: boolean;
}

export const Tab: React.FC<PropsWithChildren<TabProps>> = ({ isActive, title, children }) => {
  return (
    <p
      className={twMerge(
        "italic font-medium font-exposure transition-all relative z-10",
        isActive ? "text-black" : "text-gray-300",
      )}
    >
      {children ? children : title}
    </p>
  );
};
