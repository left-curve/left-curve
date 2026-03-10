import { motion } from "framer-motion";
import { Children, cloneElement, useRef, useState, useEffect, useCallback } from "react";
import { tv } from "tailwind-variants";
import { useControlledState } from "@left-curve/foundation";
import { twMerge } from "@left-curve/foundation";

import type React from "react";
import type { PropsWithChildren, ReactElement } from "react";
import type { VariantProps } from "tailwind-variants";
import { useHasMounted } from "../hooks/useHasMounted.js";

type TabElementProps = { title: string; isActive?: boolean; color?: string };
type TabLayout = {
  left: number;
  width: number;
  height: number;
};

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
  const containerRef = useRef<HTMLDivElement>(null);
  const tabRefs = useRef<Map<string, HTMLButtonElement>>(new Map());
  const [tabLayouts, setTabLayouts] = useState<Map<string, TabLayout>>(new Map());

  const tabs = keys ? keys : Children.toArray(children);
  const [activeTab, setActiveTab] = useControlledState(selectedTab, onTabChange, () => {
    if (defaultKey) return defaultKey;

    if (tabs.length > 0) {
      return typeof tabs[0] === "string"
        ? tabs[0]
        : (tabs[0] as ReactElement<TabElementProps>).props.title;
    }
    return "";
  });

  const measureTabs = useCallback(() => {
    if (!containerRef.current) return;
    const containerRect = containerRef.current.getBoundingClientRect();
    const newLayouts = new Map<string, TabLayout>();

    tabRefs.current.forEach((button, key) => {
      if (button) {
        const rect = button.getBoundingClientRect();
        newLayouts.set(key, {
          left: rect.left - containerRect.left,
          width: rect.width,
          height: rect.height,
        });
      }
    });

    setTabLayouts(newLayouts);
  }, []);

  useEffect(() => {
    measureTabs();
    window.addEventListener("resize", measureTabs);
    return () => window.removeEventListener("resize", measureTabs);
  }, [measureTabs, tabs.length]);

  const setTabRef = useCallback((key: string, el: HTMLButtonElement | null) => {
    if (el) {
      tabRefs.current.set(key, el);
    } else {
      tabRefs.current.delete(key);
    }
  }, []);

  const styles = tabsVariants({
    fullWidth,
    color,
    isDisabled,
  });

  const activeLayout = tabLayouts.get(activeTab);

  return (
    <div ref={containerRef} className={twMerge(styles.base(), "relative", classNames?.base)}>
      {hasMounted && activeLayout && (
        <motion.div
          initial={false}
          animate={{
            left: activeLayout.left,
            width: activeLayout.width,
          }}
          transition={{ type: "spring", bounce: 0.1, duration: 0.25 }}
          className={twMerge(styles["animated-element"](), "absolute")}
        />
      )}
      {tabs.map((e) => {
        const isKey = typeof e === "string";
        const elemKey = isKey ? e : (e as ReactElement<TabElementProps>).props.title;
        const isActive = elemKey === activeTab;

        return (
          <button
            type="button"
            ref={(el) => setTabRef(elemKey, el)}
            className={twMerge(styles.button(), { "flex-1": fullWidth }, classNames?.button)}
            key={`navLink-${elemKey}`}
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
              cloneElement(e as ReactElement<TabElementProps>, { isActive, color })
            )}
          </button>
        );
      })}
    </div>
  );
};

const tabsVariants = tv({
  slots: {
    base: "flex text-base relative items-center w-fit p-1 rounded-md exposure-sm-italic",
    button: "relative capitalize transition-all flex items-center justify-center py-2 px-4 z-10",
    "animated-element": "z-0",
  },
  variants: {
    isDisabled: {
      true: {
        button: "cursor-not-allowed opacity-50",
      },
    },
    color: {
      green: {
        base: "bg-surface-tertiary-green",
        "animated-element":
          "top-1 bottom-1 bg-surface-primary-green [box-shadow:0px_4px_6px_2px_#1919191F] rounded-[10px]",
      },
      red: {
        base: "bg-surface-secondary-red",
        "animated-element":
          "top-1 bottom-1 bg-red-400 [box-shadow:0px_4px_6px_2px_#1919191F] rounded-[10px]",
      },
      "light-green": {
        base: "bg-surface-secondary-green",
        "animated-element":
          "top-1 bottom-1 bg-surface-button-green [box-shadow:0px_4px_6px_2px_#1919191F] rounded-[10px]",
      },
      "line-red": {
        base: "p-0",
        button: "border-b-[1px] border-outline-secondary-gray pt-0",
        "animated-element": "h-[2px] top-auto bottom-[-1px] bg-primitives-red-light-400",
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
      class: "text-ink-secondary-700",
    },
    {
      isActive: false,
      color: "green",
      class: "text-fg-tertiary-400",
    },
    {
      isActive: true,
      color: "red",
      class: "text-surface-primary-rice",
    },
    {
      isActive: false,
      color: "red",
      class: "text-fg-tertiary-400",
    },
    {
      isActive: true,
      color: "light-green",
      class: "text-surface-primary-rice",
    },
    {
      isActive: false,
      color: "light-green",
      class: "text-fg-tertiary-400",
    },
    {
      isActive: true,
      color: "line-red",
      class: "text-primitives-red-light-400",
    },
    {
      isActive: false,
      color: "line-red",
      class: "text-fg-tertiary-400",
    },
  ],
});
