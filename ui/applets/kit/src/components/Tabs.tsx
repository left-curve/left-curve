import { motion } from "framer-motion";
import { Children, cloneElement } from "react";
import { tv } from "tailwind-variants";
import { useControlledState } from "#hooks/useControlledState.js";
import { twMerge } from "#utils/twMerge.js";

import type React from "react";
import type { PropsWithChildren } from "react";
import type { VariantProps } from "tailwind-variants";
import { useHasMounted } from "#hooks/useHasMounted.js";

export interface TabsProps extends VariantProps<typeof tabsVariants> {
  onTabChange?: (tab: string) => void;
  defaultKey?: string;
  keys?: string[];
  selectedTab?: string;
  layoutId: string;
  isDisabled?: boolean;
  classNames?: {
    base?: string;
    button?: string;
  };
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
  classNames,
  isDisabled,
}) => {
  const hasMounted = useHasMounted();
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
    isDisabled,
  });

  return (
    <motion.div
      layout
      layoutId={layoutId}
      initial={false}
      transition={{ duration: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className={twMerge(styles.base(), classNames?.base)}
    >
      {tabs.map((e, i) => {
        const isKey = typeof e === "string";
        const elemKey = isKey ? e : (e as React.ReactElement).props.title;
        const isActive = elemKey === activeTab;

        return (
          <motion.button
            className={twMerge(styles.button(), { "flex-1": fullWidth }, classNames?.button)}
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
            {isActive && hasMounted ? (
              <motion.div
                initial={false}
                layoutId={`active-tab-indicator-${layoutId}`}
                className={twMerge(styles["animated-element"]())}
              />
            ) : null}
          </motion.button>
        );
      })}
    </motion.div>
  );
};

const tabsVariants = tv({
  slots: {
    base: "flex text-base relative items-center w-fit p-1 rounded-md exposure-sm-italic",
    button: "relative capitalize transition-all flex items-center justify-center py-2 px-4",
    "animated-element": "absolute bottom-0 left-0",
  },
  variants: {
    isDisabled: {
      true: {
        button: "cursor-not-allowed opacity-50",
      },
    },
    color: {
      green: {
        base: "bg-secondary-green",
        "animated-element":
          "bg-button-green [box-shadow:0px_4px_6px_2px_#1919191F] w-full h-full rounded-[10px]",
      },
      red: {
        base: "bg-secondary-red",
        "animated-element":
          "bg-red-400 [box-shadow:0px_4px_6px_2px_#1919191F] w-full h-full rounded-[10px]",
      },
      "light-green": {
        base: " bg-tertiary-green",
        "animated-element":
          "bg-primary-green [box-shadow:0px_4px_6px_2px_#1919191F] w-full h-full rounded-[10px]",
      },
      "line-red": {
        base: "p-0",
        button: "border-b-[1px] border-secondary-gray pt-0",
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
  base: "transition-all relative z-10 whitespace-nowrap outline-none",
  variants: {
    color: {
      green: "",
      red: "",
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
      class: "text-secondary-700",
    },
    {
      isActive: true,
      color: "red",
      class: "text-secondary-700",
    },
    {
      isActive: false,
      color: "red",
      class: "text-foreground-tertiary",
    },
    {
      isActive: false,
      color: "green",
      class: "text-foreground-tertiary",
    },
    {
      isActive: true,
      color: "light-green",
      class: "text-secondary-700",
    },
    {
      isActive: false,
      color: "light-green",
      class: "text-foreground-tertiary",
    },
    {
      isActive: true,
      color: "line-red",
      class: "text-red-bean-400",
    },
    {
      isActive: false,
      color: "line-red",
      class: "text-foreground-tertiary",
    },
  ],
});
