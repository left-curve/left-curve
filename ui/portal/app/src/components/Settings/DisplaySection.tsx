import { m } from "@left-curve/foundation/paraglide/messages.js";
import { twMerge, useApp } from "@left-curve/foundation";
import { useTheme } from "~/hooks/useTheme";

import type { PropsWithChildren } from "react";
import type React from "react";

import {
  GlobalText,
  IconCalendar,
  IconDepth,
  IconFormatNumber,
  IconLanguage,
  IconMoon,
  IconSun,
  IconTheme,
  ShadowContainer,
  Tab,
  Tabs,
} from "../foundation";

import { View } from "react-native";

const Container: React.FC<PropsWithChildren> = ({ children }) => {
  return (
    <ShadowContainer>
      <View className="rounded-xl bg-surface-secondary-rice flex flex-col w-full px-2 py-4 gap-4">
        <GlobalText className="h4-bold text-ink-primary-900 px-2">
          {m["settings.display"]()}
        </GlobalText>
        {children}
      </View>
    </ShadowContainer>
  );
};

const LanguageSection: React.FC = () => {
  return (
    <View className="flex flex-row items-center justify-between px-2 rounded-md">
      <View className="flex flex-row items-center justify-center gap-2">
        <IconLanguage className="text-ink-tertiary-500" />
        <GlobalText className="diatype-m-bold text-ink-secondary-700">
          {m["settings.language"]()}
        </GlobalText>
      </View>

      <GlobalText>Select</GlobalText>
      {/* <Select value={getLocale()} onChange={(key) => setLocale(key as (typeof locales)[number])}>
        {locales.map((locale) => (
          <Select.Item key={locale} value={locale}>
            {m["settings.languages"]({ language: locale })}
          </Select.Item>
        ))}
      </Select> */}
    </View>
  );
};

const ChartEngineSection: React.FC = () => {
  const { settings, changeSettings } = useApp();

  const { chart } = settings;

  return (
    <View className="flex flex-row items-center justify-between px-2 rounded-md">
      <View className="flex flex-row items-center justify-center gap-2">
        <IconDepth className="text-ink-tertiary-500" />
        <GlobalText className="diatype-m-bold text-ink-secondary-700">
          {m["settings.chart"]()}
        </GlobalText>
      </View>

      <GlobalText>Select</GlobalText>

      {/* <Select value={chart} onChange={(c) => changeSettings({ chart: c as "tradingview" })}>
        {["tradingview"].map((chart) => (
          <Select.Item key={chart} value={chart}>
            {m["settings.chartEngines"]({ chart })}
          </Select.Item>
        ))}
      </Select> */}
    </View>
  );
};

const FormatNumberSection: React.FC = () => {
  const { settings, changeSettings } = useApp();
  const { formatNumberOptions } = settings;
  return (
    <View className="flex flex-row items-center justify-between px-2 rounded-md">
      <View className="flex flex-row items-center justify-center gap-2">
        <IconFormatNumber className="text-ink-tertiary-500" />
        <GlobalText className="diatype-m-bold text-ink-secondary-700">
          {m["settings.number"]()}
        </GlobalText>
      </View>

      <GlobalText>Select</GlobalText>
      {/* <Select
        value={formatNumberOptions?.mask.toString() || "1"}
        onChange={(key) => [
          changeSettings({
            formatNumberOptions: {
              ...formatNumberOptions,
              mask: Number(key) as FormatNumberOptions["mask"],
            },
          }),
        ]}
      >
        <Select.Item value="1">1,234.00</Select.Item>
        <Select.Item value="2">1.234,00</Select.Item>
        <Select.Item value="3">1234,00</Select.Item>
        <Select.Item value="4">1 234,00</Select.Item>
      </Select> */}
    </View>
  );
};

const TimeFormatSection: React.FC = () => {
  const { settings, changeSettings } = useApp();
  const { timeFormat } = settings;
  return (
    <View className="flex flex-row items-center justify-between px-2 rounded-md">
      <View className="flex flex-row items-center justify-center gap-2">
        <IconCalendar className="text-ink-tertiary-500" />
        <GlobalText className="diatype-m-bold text-ink-secondary-700">
          {m["settings.time"]()}
        </GlobalText>
      </View>

      <GlobalText>Select</GlobalText>
      {/* <Select
        value={timeFormat}
        onChange={(key) => [
          changeSettings({
            timeFormat: key as AppState["settings"]["timeFormat"],
          }),
        ]}
      >
        <Select.Item value="hh:mm a">9:18 PM</Select.Item>
        <Select.Item value="hh:mm aaa">9:18 pm</Select.Item>
        <Select.Item value="HH:mm">21:18</Select.Item>
      </Select> */}
    </View>
  );
};

const DateFormatSection: React.FC = () => {
  const { settings, changeSettings } = useApp();
  const { dateFormat } = settings;

  return (
    <View className="flex flex-row items-center justify-between px-2 rounded-md">
      <View className="flex flex-row items-center justify-center gap-2">
        <IconCalendar className="text-ink-tertiary-500" />
        <GlobalText className="diatype-m-bold text-ink-secondary-700">
          {m["settings.date"]()}
        </GlobalText>
      </View>

      <GlobalText>Select</GlobalText>

      {/*  <Select
        value={dateFormat}
        onChange={(key) => [
          changeSettings({
            dateFormat: key as AppState["settings"]["dateFormat"],
          }),
        ]}
      >
        <Select.Item value="MM/dd/yyyy">08/29/2025</Select.Item>
        <Select.Item value="dd/MM/yyyy">29/08/2025</Select.Item>
        <Select.Item value="yyyy/MM/dd">2025/08/29</Select.Item>
      </Select> */}
    </View>
  );
};

const TimeZoneSection: React.FC = () => {
  const { settings, changeSettings } = useApp();
  const { timeZone } = settings;

  return (
    <View className="flex flex-row items-center justify-between px-2 rounded-md">
      <View className="flex flex-row items-center justify-center gap-2">
        <IconCalendar className="text-ink-tertiary-500" />
        <GlobalText className="diatype-m-bold text-ink-secondary-700">
          {" "}
          {m["settings.timeZone"]()}
        </GlobalText>
      </View>

      <GlobalText>Select</GlobalText>

      {/* <Select
        value={timeZone}
        onChange={(key) => [
          changeSettings({
            timeZone: key as AppState["settings"]["timeZone"],
          }),
        ]}
      >
        <Select.Item value="utc">UTC</Select.Item>
        <Select.Item value="local">Local</Select.Item>
      </Select> */}
    </View>
  );
};

const ThemeSection: React.FC = () => {
  const { themeSchema, setThemeSchema } = useTheme();

  return (
    <View className="flex flex-row items-center justify-between px-[10px] py-2 rounded-md">
      <View className="flex flex-row items-center justify-center gap-2">
        <IconTheme className="text-ink-tertiary-500" />
        <GlobalText className="diatype-m-bold text-ink-secondary-700">
          {m["settings.theme"]()}
        </GlobalText>
      </View>
      <Tabs
        selectedTab={themeSchema}
        onTabChange={(value) => setThemeSchema(value as "system" | "light" | "dark")}
        classNames={{ base: "exposure-sm-italic" }}
      >
        <Tab title="system">
          <GlobalText>{m["settings.system"]()}</GlobalText>
        </Tab>
        <Tab title="light">
          <IconSun
            className={twMerge(
              themeSchema === "light" ? "text-ink-secondary-700" : "text-fg-tertiary-400",
            )}
          />
        </Tab>
        <Tab title="dark">
          <IconMoon
            className={twMerge(
              themeSchema === "dark" ? "text-ink-secondary-700" : "text-fg-tertiary-400",
            )}
          />
        </Tab>
      </Tabs>
    </View>
  );
};

export const DisplaySection = Object.assign(Container, {
  Language: LanguageSection,
  ChartEngine: ChartEngineSection,
  FormatNumber: FormatNumberSection,
  TimeFormat: TimeFormatSection,
  TimeZone: TimeZoneSection,
  DateFormat: DateFormatSection,
  Theme: ThemeSection,
});
