import { type ReactNode, useCallback } from "react";
import { View, Text, Pressable } from "react-native";
import { twMerge } from "@left-curve/foundation";

type DropdownAlign = "left" | "right";

export type DropdownProps = {
  readonly trigger: ReactNode;
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
  readonly children: ReactNode;
  readonly align?: DropdownAlign;
  readonly className?: string;
};

export function Dropdown({
  trigger,
  open,
  onOpenChange,
  children,
  align = "left",
  className,
}: DropdownProps) {
  const handleDismiss = useCallback(() => onOpenChange(false), [onOpenChange]);

  return (
    <View style={{ position: "relative" as never, zIndex: open ? 99999 : 0 }}>
      {trigger}
      {open && (
        <>
          {/* Full-screen overlay to catch outside clicks */}
          <Pressable
            onPress={handleDismiss}
            style={{
              position: "fixed" as never,
              top: 0,
              left: 0,
              right: 0,
              bottom: 0,
              zIndex: 99998,
            }}
            aria-label="Close dropdown"
          />
          <View
            className={twMerge(
              "flex flex-col bg-bg-elev border border-border-default rounded-field overflow-hidden shadow-lg",
              className,
            )}
            style={{
              position: "absolute" as never,
              top: "100%" as never,
              marginTop: 4,
              ...(align === "right" ? { right: 0 } : { left: 0 }),
              zIndex: 99999,
              minWidth: 64,
            }}
          >
            {children}
          </View>
        </>
      )}
    </View>
  );
}

export type DropdownItemProps = {
  readonly onPress: () => void;
  readonly selected?: boolean;
  readonly children: ReactNode;
  readonly className?: string;
};

export function DropdownItem({
  onPress,
  selected = false,
  children,
  className,
}: DropdownItemProps) {
  return (
    <Pressable
      onPress={onPress}
      className={twMerge(
        "flex flex-row items-center justify-between px-3 py-2 hover:bg-bg-sunk",
        selected && "bg-bg-sunk",
        className,
      )}
    >
      <View className="flex flex-1 flex-col">{children}</View>
      {selected && <Text className="text-accent text-[12px] ml-2">{"\u2713"}</Text>}
    </Pressable>
  );
}
