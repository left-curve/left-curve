import type React from "react";
import { Children, cloneElement, useMemo, useRef, useState, useEffect } from "react";
import { View, Pressable, Text, Animated, type LayoutChangeEvent } from "react-native";
import { tv, type VariantProps } from "tailwind-variants";
import { twMerge } from "@left-curve/foundation";
import { useControlledState } from "@left-curve/foundation";

type ReactChildren = React.PropsWithChildren<{}>["children"];

export interface TabsProps extends VariantProps<typeof tabsVariants> {
  onTabChange?: (tab: string) => void;
  defaultKey?: string;
  keys?: string[];
  selectedTab?: string;
  isDisabled?: boolean;
  classNames?: {
    base?: string;
    button?: string;
  };
}

export const Tabs: React.FC<React.PropsWithChildren<TabsProps>> = ({
  onTabChange,
  children,
  keys,
  selectedTab,
  defaultKey,
  fullWidth,
  color,
  classNames,
  isDisabled,
}) => {
  const tabs = useMemo(
    () => (keys ? keys : Children.toArray(children as ReactChildren)),
    [keys, children],
  );

  const [activeTab, setActiveTab] = useControlledState(selectedTab, onTabChange, () => {
    if (defaultKey) return defaultKey;
    if (tabs.length > 0) {
      return typeof tabs[0] === "string"
        ? (tabs[0] as string)
        : (tabs[0] as React.ReactElement).props.title;
    }
    return "";
  });

  const styles = tabsVariants({ fullWidth, color, isDisabled });

  type Layout = { x: number; width: number; height: number };
  const [layouts, setLayouts] = useState<Record<string, Layout>>({});
  const left = useRef(new Animated.Value(0)).current;
  const width = useRef(new Animated.Value(0)).current;
  const height = useRef(new Animated.Value(0)).current;
  const isLine = color === "line-red";

  const onTabLayout =
    (key: string) =>
    (e: LayoutChangeEvent): void => {
      const { x, width, height } = e.nativeEvent.layout;
      setLayouts((prev) => ({ ...prev, [key]: { x, width, height } }));
    };

  useEffect(() => {
    const l = layouts[activeTab];
    if (!l) return;

    const duration = 60;
    Animated.timing(left, { toValue: l.x, duration, useNativeDriver: false }).start();
    Animated.timing(width, { toValue: l.width, duration, useNativeDriver: false }).start();
    Animated.timing(height, {
      toValue: isLine ? 2 : l.height,
      duration,
      useNativeDriver: false,
    }).start();
  }, [activeTab, layouts, left, width, height]);

  return (
    <View className={twMerge(styles.base(), classNames?.base)}>
      {layouts[activeTab] ? (
        <Animated.View
          pointerEvents="none"
          className={twMerge(styles["animated-element"]())}
          style={{
            position: "absolute",
            left,
            width,
            height,
          }}
        />
      ) : null}

      {tabs.map((e) => {
        const isKey = typeof e === "string";
        const elemKey = isKey ? (e as string) : (e as React.ReactElement).props.title;
        const isActive = elemKey === activeTab;

        return (
          <Pressable
            key={`navLink-${elemKey}`}
            accessibilityLabel={`tabs-${elemKey}`}
            disabled={!!isDisabled}
            onPress={() => setActiveTab(elemKey)}
            onLayout={onTabLayout(elemKey)}
            className={twMerge(styles.button(), { "flex-1": fullWidth }, classNames?.button)}
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
              cloneElement(e as React.ReactElement, { isActive, color })
            )}
          </Pressable>
        );
      })}
    </View>
  );
};

const tabsVariants = tv({
  slots: {
    base: "flex flex-row text-base relative items-center p-1 rounded-md exposure-sm-italic",
    button: "relative capitalize transition-all flex items-center justify-center py-2 px-4",
    "animated-element": "absolute w-full rounded-[10px]",
  },
  variants: {
    isDisabled: {
      true: {
        button: "opacity-50",
      },
    },
    color: {
      green: {
        base: "bg-surface-tertiary-green",
        "animated-element": "bg-surface-primary-green h-full top-1 left-0 ",
      },
      red: {
        base: "bg-surface-secondary-red",
        "animated-element": "bg-red-400 h-full top-1 left-0 ",
      },
      "light-green": {
        base: "bg-surface-secondary-green",
        "animated-element": "bg-surface-button-green h-full top-1 left-0 ",
      },
      "line-red": {
        base: "p-0",
        button: "border-b-[1px] border-outline-secondary-gray pt-0",
        "animated-element": "bg-primitives-red-light-400 w-full h-[2px] bottom-[-1px]",
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

export const Tab: React.FC<React.PropsWithChildren<TabProps>> = ({
  isActive,
  color,
  fullWidth,
  title,
  children,
}) => {
  const styles = tabVariants({ color, isActive, fullWidth });
  return <Text className={twMerge(styles)}>{children ? children : title}</Text>;
};

const tabVariants = tv({
  base: "transition-all relative z-10 whitespace-nowrap outline-none",
  variants: {
    color: { green: "", red: "", "light-green": "", "line-red": "" },
    fullWidth: { true: "flex-1", false: "" },
    isActive: { true: "", false: "" },
  },
  defaultVariants: {
    fullWidth: false,
    color: "green",
    isActive: false,
  },
  compoundVariants: [
    { isActive: true, color: "green", class: "text-ink-secondary-700" },
    { isActive: false, color: "green", class: "text-fg-tertiary-400" },
    { isActive: true, color: "red", class: "text-surface-primary-rice" },
    { isActive: false, color: "red", class: "text-fg-tertiary-400" },
    { isActive: true, color: "light-green", class: "text-surface-primary-rice" },
    { isActive: false, color: "light-green", class: "text-fg-tertiary-400" },
    { isActive: true, color: "line-red", class: "text-primitives-red-light-400" },
    { isActive: false, color: "line-red", class: "text-fg-tertiary-400" },
  ],
});
