import {
  IconCalendar,
  IconDepth,
  IconFormatNumber,
  IconLanguage,
  IconMoon,
  IconSun,
  IconTheme,
  IconTime,
  IconWorld,
  Tab,
  Tabs,
  useApp,
  useTheme,
} from "@left-curve/applets-kit";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { getLocale, locales, setLocale } from "@left-curve/foundation/paraglide/runtime.js";

import { SettingSelect } from "./SettingSelect";

import type { FormatNumberOptions } from "@left-curve/dango/utils";
import type { PropsWithChildren } from "react";
import type React from "react";
import type { AppState } from "@left-curve/applets-kit";

const Container: React.FC<PropsWithChildren> = ({ children }) => {
  return (
    <div className="rounded-xl bg-surface-secondary-rice shadow-account-card flex flex-col w-full px-2 py-2 ">
      {children}
    </div>
  );
};

const LanguageSection: React.FC = () => {
  const options = locales.map((locale) => ({
    value: locale,
    label: m["settings.languages"]({ language: locale }),
  }));

  return (
    <SettingSelect
      value={getLocale()}
      onChange={(key) => setLocale(key as (typeof locales)[number])}
      options={options}
      icon={<IconLanguage className="text-ink-tertiary-500" />}
      label={m["settings.language"]()}
    />
  );
};

const ChartEngineSection: React.FC = () => {
  const { settings, changeSettings } = useApp();
  const { chart } = settings;

  const options = ["tradingview"].map((c) => ({
    value: c,
    label: m["settings.chartEngines"]({ chart: c }),
  }));

  return (
    <SettingSelect
      value={chart}
      onChange={(c) => changeSettings({ chart: c as "tradingview" })}
      options={options}
      icon={<IconDepth className="text-ink-tertiary-500" />}
      label={m["settings.chart"]()}
    />
  );
};

const FormatNumberSection: React.FC = () => {
  const { settings, changeSettings } = useApp();
  const { formatNumberOptions } = settings;

  const currentValue = formatNumberOptions?.mask.toString() || "1";

  const options = [
    { value: "1", label: "1,234.56" },
    { value: "2", label: "1.234,56" },
    { value: "3", label: "1234,56" },
    { value: "4", label: "1 234,56" },
  ];

  return (
    <SettingSelect
      value={currentValue}
      onChange={(key) =>
        changeSettings({
          formatNumberOptions: {
            ...formatNumberOptions,
            mask: Number(key) as FormatNumberOptions["mask"],
          },
        })
      }
      options={options}
      icon={<IconFormatNumber className="text-ink-tertiary-500" />}
      label={m["settings.number"]()}
    />
  );
};

const TimeFormatSection: React.FC = () => {
  const { settings, changeSettings } = useApp();
  const { timeFormat } = settings;

  const options = [
    { value: "hh:mm a", label: "9:18 PM" },
    { value: "hh:mm aaa", label: "9:18 pm" },
    { value: "HH:mm", label: "21:18" },
  ];

  return (
    <SettingSelect
      value={timeFormat}
      onChange={(key) =>
        changeSettings({
          timeFormat: key as AppState["settings"]["timeFormat"],
        })
      }
      options={options}
      icon={<IconTime className="text-ink-tertiary-500" />}
      label={m["settings.time"]()}
    />
  );
};

const DateFormatSection: React.FC = () => {
  const { settings, changeSettings } = useApp();
  const { dateFormat } = settings;

  const options = [
    { value: "MM/dd/yyyy", label: "08/29/2025" },
    { value: "dd/MM/yyyy", label: "29/08/2025" },
    { value: "yyyy/MM/dd", label: "2025/08/29" },
    { value: "dd MMM yyyy", label: "16 Sep 2025" },
  ];

  return (
    <SettingSelect
      value={dateFormat}
      onChange={(key) =>
        changeSettings({
          dateFormat: key as AppState["settings"]["dateFormat"],
        })
      }
      options={options}
      icon={<IconCalendar className="text-ink-tertiary-500" />}
      label={m["settings.date"]()}
    />
  );
};

const TimeZoneSection: React.FC = () => {
  const { settings, changeSettings } = useApp();
  const { timeZone } = settings;

  const options = [
    { value: "utc", label: "UTC" },
    { value: "local", label: "Local" },
  ];

  return (
    <SettingSelect
      value={timeZone}
      onChange={(key) =>
        changeSettings({
          timeZone: key as AppState["settings"]["timeZone"],
        })
      }
      options={options}
      icon={<IconWorld className="text-ink-tertiary-500" />}
      label={m["settings.timeZone"]()}
    />
  );
};

const ThemeSection: React.FC = () => {
  const { themeSchema, setThemeSchema } = useTheme();

  return (
    <div className="flex items-center justify-between px-[10px] py-2 rounded-md">
      <p className="flex items-center justify-center gap-2">
        <IconTheme className="text-ink-tertiary-500" />
        <span className="diatype-m-bold text-ink-secondary-700">{m["settings.theme"]()}</span>
      </p>
      <Tabs
        selectedTab={themeSchema}
        layoutId="theme"
        onTabChange={(value) => setThemeSchema(value as "system" | "light" | "dark")}
        classNames={{ base: "exposure-sm-italic md:min-w-[12.375rem]" }}
      >
        <Tab title="system">System</Tab>
        <Tab title="light">
          <IconSun className="w-6 h-6" />
        </Tab>
        <Tab title="dark">
          <IconMoon className="w-6 h-6" />
        </Tab>
      </Tabs>
    </div>
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
