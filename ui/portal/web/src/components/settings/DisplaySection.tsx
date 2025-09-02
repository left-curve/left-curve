import {
  IconDepth,
  IconFormatNumber,
  IconLanguage,
  IconMoon,
  IconSun,
  IconTheme,
  Select,
  Tab,
  Tabs,
  useApp,
  useTheme,
} from "@left-curve/applets-kit";

import { m } from "~/paraglide/messages";
import { getLocale, locales, setLocale } from "~/paraglide/runtime";

import type { FormatNumberOptions } from "@left-curve/dango/utils";
import type { PropsWithChildren } from "react";
import type React from "react";

const Container: React.FC<PropsWithChildren> = ({ children }) => {
  return (
    <div className="rounded-xl bg-surface-secondary-rice shadow-account-card flex flex-col w-full px-2 py-4 gap-4">
      <h3 className="h4-bold text-primary-900 px-2">{m["settings.display"]()}</h3>
      {children}
    </div>
  );
};

const LanguageSection: React.FC = () => {
  return (
    <div className="flex items-center justify-between px-2 rounded-md">
      <p className="flex items-center justify-center gap-2">
        <IconLanguage className="text-tertiary-500" />
        <span className="diatype-m-bold text-secondary-700">{m["settings.language"]()}</span>
      </p>
      <Select value={getLocale()} onChange={(key) => setLocale(key as (typeof locales)[number])}>
        {locales.map((locale) => (
          <Select.Item key={locale} value={locale}>
            {m["settings.languages"]({ language: locale })}
          </Select.Item>
        ))}
      </Select>
    </div>
  );
};

const ChartEngineSection: React.FC = () => {
  const { settings, changeSettings } = useApp();

  const { chart } = settings;

  return (
    <div className="flex items-center justify-between px-2 rounded-md">
      <p className="flex items-center justify-center gap-2">
        <IconDepth className="text-tertiary-500" />
        <span className="diatype-m-bold text-secondary-700">{m["settings.chart"]()}</span>
      </p>
      <Select
        value={chart}
        onChange={(c) => changeSettings({ chart: c as "tradingview" | "chartiq" })}
      >
        {["tradingview", "chartiq"].map((chart) => (
          <Select.Item key={chart} value={chart}>
            {m["settings.chartEngines"]({ chart })}
          </Select.Item>
        ))}
      </Select>
    </div>
  );
};

const FormatNumberSection: React.FC = () => {
  const { settings, changeSettings } = useApp();
  const { formatNumberOptions } = settings;
  return (
    <div className="flex items-center justify-between px-2 rounded-md">
      <p className="flex items-center justify-center gap-2">
        <IconFormatNumber className="text-tertiary-500" />
        <span className="diatype-m-bold text-secondary-700"> {m["settings.number"]()}</span>
      </p>

      <Select
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
      </Select>
    </div>
  );
};

const ThemeSection: React.FC = () => {
  const { themeSchema, setThemeSchema } = useTheme();

  return (
    <div className="flex items-center justify-between px-[10px] py-2 rounded-md">
      <p className="flex items-center justify-center gap-2">
        <IconTheme className="text-tertiary-500" />
        <span className="diatype-m-bold text-secondary-700">{m["settings.theme"]()}</span>
      </p>
      <Tabs
        selectedTab={themeSchema}
        layoutId="theme"
        onTabChange={(value) => setThemeSchema(value as "system" | "light" | "dark")}
        classNames={{ base: "exposure-sm-italic" }}
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
  Theme: ThemeSection,
});
