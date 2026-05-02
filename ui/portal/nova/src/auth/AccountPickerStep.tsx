import { Pressable, Text, View } from "react-native";
import { twMerge } from "@left-curve/foundation";
import { Button, Spinner } from "../components";

import type { User } from "@left-curve/dango/types";

type AccountPickerStepProps = {
  readonly users: readonly User[];
  readonly onSelectAccount: (userIndex: number) => void;
  readonly onCreateNew: () => void;
  readonly onBack: () => void;
  readonly isSelecting: boolean;
  readonly isCreating: boolean;
};

export function AccountPickerStep({
  users,
  onSelectAccount,
  onCreateNew,
  onBack,
  isSelecting,
  isCreating,
}: AccountPickerStepProps) {
  const isBusy = isSelecting || isCreating;

  return (
    <View className="flex flex-col gap-6 px-6 pb-6 pt-2">
      <View className="flex flex-col gap-2 items-center">
        <Text
          className={twMerge(
            "font-display font-bold",
            "text-[20px] text-fg-primary",
            "text-center",
          )}
        >
          Welcome back
        </Text>
        <Text className={twMerge("font-text text-[13px]", "text-fg-secondary text-center")}>
          Choose which account to sign in with.
        </Text>
      </View>

      <View className="flex flex-col gap-2">
        {users.map((user) => (
          <Pressable
            key={user.index}
            onPress={() => onSelectAccount(user.index)}
            disabled={isBusy}
            className={twMerge(
              "flex-row items-center gap-3",
              "px-3.5 py-3",
              "bg-bg-surface",
              "border border-border-subtle",
              "rounded-field",
              "hover:border-border-strong hover:bg-bg-tint",
              "transition-[background,border-color] duration-150 ease-[var(--ease)]",
              isBusy && "opacity-55 pointer-events-none",
            )}
          >
            {/* Avatar */}
            <View
              className={twMerge(
                "w-8 h-8 rounded-full",
                "items-center justify-center",
                "bg-accent-bg",
              )}
            >
              <Text className="text-accent text-[12px] font-semibold">
                {(user.name ?? `#${user.index}`).charAt(0).toUpperCase()}
              </Text>
            </View>

            <View className="flex-1 min-w-0">
              <Text className="text-fg-primary text-[14px] font-medium" numberOfLines={1}>
                {user.name ?? `Account #${user.index}`}
              </Text>
            </View>

            <Text className="text-fg-quaternary text-[14px]">{"\u203A"}</Text>
          </Pressable>
        ))}
      </View>

      <Button variant="secondary" size="lg" onPress={onCreateNew} disabled={isBusy}>
        {isCreating ? (
          <View className="flex-row items-center gap-2">
            <Spinner size="sm" className="text-fg-primary" />
            <Text className="font-medium text-fg-primary text-[14px]">Creating...</Text>
          </View>
        ) : (
          <Text className="font-medium text-fg-primary text-[14px]">Create New Account</Text>
        )}
      </Button>

      <View className="items-center">
        <Pressable onPress={onBack} disabled={isBusy}>
          <Text className="text-fg-tertiary text-[12px] font-medium">Back</Text>
        </Pressable>
      </View>
    </View>
  );
}
