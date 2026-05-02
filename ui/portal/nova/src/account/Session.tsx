import { View, Text } from "react-native";
import { useCountdown } from "@left-curve/foundation";
import { useAccount, useSessionKey } from "@left-curve/store";
import { Button, Card, Dot } from "../components";

function formatSessionTime(timestamp: number): string {
  const date = new Date(timestamp * 1000);
  return date.toLocaleString("en-US", {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    timeZoneName: "short",
  });
}

export function Session() {
  const { isConnected, connector } = useAccount();
  const { session, deleteSessionKey } = useSessionKey();

  const expireAt = Number(session?.sessionInfo.expireAt ?? 0);

  const { hours, minutes, seconds } = useCountdown({
    date: expireAt * 1000,
    showLeadingZeros: true,
  });

  const isActive = isConnected && !!session && Date.now() < expireAt * 1000;
  const truncatedSessionKey = session?.sessionInfo.sessionKey
    ? `${session.sessionInfo.sessionKey.slice(0, 8)}...${session.sessionInfo.sessionKey.slice(-4)}`
    : "";

  return (
    <View className="flex flex-col gap-4">
      <Text className="text-fg-primary text-[20px] font-display font-semibold tracking-tight">
        Session
      </Text>

      <Card className="p-5">
        <View className="flex flex-row items-center justify-between">
          <Text className="text-fg-primary text-[14px] font-semibold tracking-tight">
            Current session
          </Text>
          {isActive ? (
            <View className="flex flex-row items-center gap-1.5">
              <Dot variant="up" pulse />
              <Text className="text-up text-[12px] font-medium">Active</Text>
            </View>
          ) : (
            <View className="flex flex-row items-center gap-1.5">
              <Dot variant="default" />
              <Text className="text-fg-tertiary text-[12px] font-medium">
                {session ? "Expired" : "No session"}
              </Text>
            </View>
          )}
        </View>

        <View className="flex flex-col gap-3 mt-4">
          <View className="flex flex-row items-center justify-between py-2">
            <Text className="text-fg-tertiary text-[13px]">Expires</Text>
            {session ? (
              <Text className="text-fg-primary text-[13px] tabular-nums">
                {formatSessionTime(expireAt)}
              </Text>
            ) : (
              <Text className="text-fg-tertiary text-[13px]">{"\u2014"}</Text>
            )}
          </View>

          <View className="h-px bg-border-subtle" />

          <View className="flex flex-row items-center justify-between py-2">
            <Text className="text-fg-tertiary text-[13px]">Session key</Text>
            {session ? (
              <Text className="text-fg-secondary text-[13px] font-mono">{truncatedSessionKey}</Text>
            ) : (
              <Text className="text-fg-tertiary text-[13px]">{"\u2014"}</Text>
            )}
          </View>

          <View className="h-px bg-border-subtle" />

          <View className="flex flex-row items-center justify-between py-2">
            <Text className="text-fg-tertiary text-[13px]">Authorization</Text>
            <Text className="text-fg-secondary text-[13px]">{connector?.name ?? "\u2014"}</Text>
          </View>
        </View>
      </Card>

      <Card className="p-5">
        <Text className="text-fg-primary text-[14px] font-semibold tracking-tight">
          Session countdown
        </Text>
        <Text className="text-fg-tertiary text-[12px] mt-1">
          Time remaining until your session expires and requires re-authentication.
        </Text>

        <View className="mt-4 p-4 bg-bg-sunk rounded-field items-center">
          {isActive ? (
            <>
              <Text className="text-fg-primary font-mono text-[28px] font-semibold tabular-nums tracking-tight">
                {hours}:{minutes}:{seconds}
              </Text>
              <Text className="text-fg-tertiary text-[11px] mt-1">hours remaining</Text>
            </>
          ) : (
            <Text className="text-fg-tertiary text-[13px]">
              {session ? "Session expired" : "No active session"}
            </Text>
          )}
        </View>
      </Card>

      {session && (
        <View>
          <Button variant="secondary" size="lg" onPress={deleteSessionKey}>
            <Text className="text-down text-[14px] font-medium">Disconnect Session</Text>
          </Button>
        </View>
      )}
    </View>
  );
}
