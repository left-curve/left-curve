import { motion } from "framer-motion";
import { Children, cloneElement } from "react";
import { useControlledState } from "#hooks/useControlledState.js";
import { twMerge } from "#utils/twMerge.js";
import { tv } from "tailwind-variants";

import type React from "react";
import type { PropsWithChildren } from "react";
import type { VariantProps } from "tailwind-variants";

export interface TabsProps extends VariantProps<typeof tabsVariants> {
  onTabChange?: (tab: string) => void;
  defaultKey?: string;
  keys?: string[];
  selectedTab?: string;
  layoutId: string;
}

export const Tabs: React.FC<PropsWithChildren<TabsProps>> = ({
  onTabChange,
  children,
  keys,
  selectedTab,
  defaultKey,
  fullWidth,
  layoutId,
  color,
}) => {
  const tabs = keys ? keys : Children.toArray(children);
  const [activeTab, setActiveTab] = useControlledState(selectedTab, onTabChange, () => {
    if (defaultKey) return defaultKey;

    if (tabs.length > 0) {
      return typeof tabs[0] === "string" ? tabs[0] : (tabs[0] as React.ReactElement).props.title;
    }
    return "";
  });

  const styles = tabsVariants({
    fullWidth,
    color,
  });

  return (
    <motion.div layoutId={layoutId} className={twMerge(styles.base())}>
      {tabs.map((e, i) => {
        const isKey = typeof e === "string";
        const elemKey = isKey ? e : (e as React.ReactElement).props.title;
        const isActive = elemKey === activeTab;

        return (
          <motion.button
            className={twMerge(styles.button(), { "flex-1": fullWidth })}
            key={`navLink-${e}`}
            onClick={() => setActiveTab(elemKey)}
          >
            {isKey ? (
              <Tab
                key={elemKey}
                title={elemKey}
                isActive={isActive}
                color={color}
                fullWidth={fullWidth}
              />
            ) : (
              cloneElement(e as React.ReactElement, { isActive })
            )}
            {isActive ? (
              <motion.div className={twMerge(styles["animated-element"]())} layoutId="active" />
            ) : null}
          </motion.button>
        );
      })}
    </motion.div>
  );
};

const tabsVariants = tv({
  slots: {
    base: "flex text-base relative items-center w-fit  p-1 rounded-md",
    button:
      "relative capitalize transition-all flex items-center justify-center py-2 px-4 cursor-pointer",
    "animated-element": "absolute bottom-0 left-0",
  },
  variants: {
    color: {
      green: {
        base: "bg-green-bean-200",
        "animated-element":
          "bg-green-bean-50 [box-shadow:0px_4px_6px_2px_#1919191F] w-full h-full rounded-[10px]",
      },
      "light-green": {
        base: "bg-green-bean-100",
        "animated-element":
          "bg-green-bean-400 [box-shadow:0px_4px_6px_2px_#1919191F] w-full h-full rounded-[10px]",
      },
      "line-red": {
        base: "",
        button: "border-b-[1px] border-gray-100",
        "animated-element": "bg-red-bean-400 w-full h-[2px] bottom-[-1px]",
      },
    },
    fullWidth: {
      true: "w-full",
      false: "",
    },
  },

  defaultVariants: {
    fullWidth: false,
    color: "green",
  },
});

export interface TabProps extends VariantProps<typeof tabVariants> {
  title: string;
}

export const Tab: React.FC<PropsWithChildren<TabProps>> = ({
  isActive,
  color,
  fullWidth,
  title,
  children,
}) => {
  const styles = tabVariants({
    color,
    isActive,
    fullWidth,
  });
  return <p className={twMerge(styles)}>{children ? children : title}</p>;
};

const tabVariants = tv({
  base: "italic font-medium font-exposure transition-all relative z-10",
  variants: {
    color: {
      green: "",
      "light-green": "",
      "line-red": "",
    },
    fullWidth: {
      true: "flex-1",
      false: "",
    },
    isActive: {
      true: "",
      false: "",
    },
  },
  defaultVariants: {
    fullWidth: false,
    color: "green",
    isActive: false,
  },
  compoundVariants: [
    {
      isActive: true,
      color: "green",
      class: "text-black",
    },
    {
      isActive: false,
      color: "green",
      class: "text-gray-300",
    },
    {
      isActive: true,
      color: "light-green",
      class: "text-white-100",
    },
    {
      isActive: false,
      color: "light-green",
      class: "text-gray-300",
    },
    {
      isActive: true,
      color: "line-red",
      class: "text-red-bean-400",
    },
    {
      isActive: false,
      color: "line-red",
      class: "text-gray-300",
    },
  ],
});
