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
  Select,
  Tab,
  Tabs,
  useApp,
  useMediaQuery,
  useTheme,
} from "@left-curve/applets-kit";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { getLocale, locales, setLocale } from "@left-curve/foundation/paraglide/runtime.js";

import { useRef } from "react";

import type { FormatNumberOptions } from "@left-curve/dango/utils";
import type { PropsWithChildren } from "react";
import type React from "react";
import type { AppState, SelectRef } from "@left-curve/applets-kit";

const Container: React.FC<PropsWithChildren> = ({ children }) => {
  return (
    <div className="rounded-xl bg-surface-secondary-rice shadow-account-card flex flex-col w-full px-2 py-2">
      {children}
    </div>
  );
};

const LanguageSection: React.FC = () => {
  const { isMd } = useMediaQuery();
  const selectRef = useRef<SelectRef>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  if (isMd) {
    return (
      <div
        ref={containerRef}
        className="flex items-center justify-between px-2 py-2 rounded-md cursor-pointer hover:bg-surface-tertiary-rice transition-all"
        onClick={() => selectRef.current?.toggle()}
      >
        <p className="flex items-center justify-center gap-2">
          <IconLanguage className="text-ink-tertiary-500" />
          <span className="diatype-m-bold text-ink-secondary-700">{m["settings.language"]()}</span>
        </p>
        <Select
          ref={selectRef}
          containerRef={containerRef}
          value={getLocale()}
          onChange={(key) => setLocale(key as (typeof locales)[number])}
        >
          {locales.map((locale) => (
            <Select.Item key={locale} value={locale}>
              {m["settings.languages"]({ language: locale })}
            </Select.Item>
          ))}
        </Select>
      </div>
    );
  }

  return (
    <div className="flex items-center justify-between px-2 py-2 rounded-md">
      <p className="flex items-center justify-center gap-2">
        <IconLanguage className="text-ink-tertiary-500" />
        <span className="diatype-m-bold text-ink-secondary-700">{m["settings.language"]()}</span>
      </p>
      <Select
        value={getLocale()}
        onChange={(key) => setLocale(key as (typeof locales)[number])}
      >
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
  const { isMd } = useMediaQuery();
  const { settings, changeSettings } = useApp();
  const selectRef = useRef<SelectRef>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const { chart } = settings;

  if (isMd) {
    return (
      <div
        ref={containerRef}
        className="flex items-center justify-between px-2 py-2 rounded-md cursor-pointer hover:bg-surface-tertiary-rice transition-all"
        onClick={() => selectRef.current?.toggle()}
      >
        <p className="flex items-center justify-center gap-2">
          <IconDepth className="text-ink-tertiary-500" />
          <span className="diatype-m-bold text-ink-secondary-700">{m["settings.chart"]()}</span>
        </p>
        <Select
          ref={selectRef}
          containerRef={containerRef}
          value={chart}
          onChange={(c) => changeSettings({ chart: c as "tradingview" })}
        >
          {["tradingview"].map((c) => (
            <Select.Item key={c} value={c}>
              {m["settings.chartEngines"]({ chart: c })}
            </Select.Item>
          ))}
        </Select>
      </div>
    );
  }

  return (
    <div className="flex items-center justify-between px-2 py-2 rounded-md">
      <p className="flex items-center justify-center gap-2">
        <IconDepth className="text-ink-tertiary-500" />
        <span className="diatype-m-bold text-ink-secondary-700">{m["settings.chart"]()}</span>
      </p>
      <Select
        value={chart}
        onChange={(c) => changeSettings({ chart: c as "tradingview" })}
      >
        {["tradingview"].map((c) => (
          <Select.Item key={c} value={c}>
            {m["settings.chartEngines"]({ chart: c })}
          </Select.Item>
        ))}
      </Select>
    </div>
  );
};

const FormatNumberSection: React.FC = () => {
  const { isMd } = useMediaQuery();
  const { settings, changeSettings } = useApp();
  const { formatNumberOptions } = settings;
  const selectRef = useRef<SelectRef>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const currentValue = formatNumberOptions?.mask.toString() || "1";

  if (isMd) {
    return (
      <div
        ref={containerRef}
        className="flex items-center justify-between px-2 py-2 rounded-md cursor-pointer hover:bg-surface-tertiary-rice transition-all"
        onClick={() => selectRef.current?.toggle()}
      >
        <p className="flex items-center justify-center gap-2">
          <IconFormatNumber className="text-ink-tertiary-500" />
          <span className="diatype-m-bold text-ink-secondary-700"> {m["settings.number"]()}</span>
        </p>
        <Select
          ref={selectRef}
          containerRef={containerRef}
          value={currentValue}
          onChange={(key) =>
            changeSettings({
              formatNumberOptions: {
                ...formatNumberOptions,
                mask: Number(key) as FormatNumberOptions["mask"],
              },
            })
          }
        >
          <Select.Item value="1">1,234.00</Select.Item>
          <Select.Item value="2">1.234,00</Select.Item>
          <Select.Item value="3">1234,00</Select.Item>
          <Select.Item value="4">1 234,00</Select.Item>
        </Select>
      </div>
    );
  }

  return (
    <div className="flex items-center justify-between px-2 py-2 rounded-md">
      <p className="flex items-center justify-center gap-2">
        <IconFormatNumber className="text-ink-tertiary-500" />
        <span className="diatype-m-bold text-ink-secondary-700"> {m["settings.number"]()}</span>
      </p>
      <Select
        value={currentValue}
        onChange={(key) =>
          changeSettings({
            formatNumberOptions: {
              ...formatNumberOptions,
              mask: Number(key) as FormatNumberOptions["mask"],
            },
          })
        }
      >
        <Select.Item value="1">1,234.00</Select.Item>
        <Select.Item value="2">1.234,00</Select.Item>
        <Select.Item value="3">1234,00</Select.Item>
        <Select.Item value="4">1 234,00</Select.Item>
      </Select>
    </div>
  );
};

const TimeFormatSection: React.FC = () => {
  const { isMd } = useMediaQuery();
  const { settings, changeSettings } = useApp();
  const { timeFormat } = settings;
  const selectRef = useRef<SelectRef>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  if (isMd) {
    return (
      <div
        ref={containerRef}
        className="flex items-center justify-between px-2 py-2 rounded-md cursor-pointer hover:bg-surface-tertiary-rice transition-all"
        onClick={() => selectRef.current?.toggle()}
      >
        <p className="flex items-center justify-center gap-2">
          <IconTime className="text-ink-tertiary-500" />
          <span className="diatype-m-bold text-ink-secondary-700"> {m["settings.time"]()}</span>
        </p>
        <Select
          ref={selectRef}
          containerRef={containerRef}
          value={timeFormat}
          onChange={(key) =>
            changeSettings({
              timeFormat: key as AppState["settings"]["timeFormat"],
            })
          }
        >
          <Select.Item value="hh:mm a">9:18 PM</Select.Item>
          <Select.Item value="hh:mm aaa">9:18 pm</Select.Item>
          <Select.Item value="HH:mm">21:18</Select.Item>
        </Select>
      </div>
    );
  }

  return (
    <div className="flex items-center justify-between px-2 py-2 rounded-md">
      <p className="flex items-center justify-center gap-2">
        <IconTime className="text-ink-tertiary-500" />
        <span className="diatype-m-bold text-ink-secondary-700"> {m["settings.time"]()}</span>
      </p>
      <Select
        value={timeFormat}
        onChange={(key) =>
          changeSettings({
            timeFormat: key as AppState["settings"]["timeFormat"],
          })
        }
      >
        <Select.Item value="hh:mm a">9:18 PM</Select.Item>
        <Select.Item value="hh:mm aaa">9:18 pm</Select.Item>
        <Select.Item value="HH:mm">21:18</Select.Item>
      </Select>
    </div>
  );
};

const DateFormatSection: React.FC = () => {
  const { isMd } = useMediaQuery();
  const { settings, changeSettings } = useApp();
  const { dateFormat } = settings;
  const selectRef = useRef<SelectRef>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  if (isMd) {
    return (
      <div
        ref={containerRef}
        className="flex items-center justify-between px-2 py-2 rounded-md cursor-pointer hover:bg-surface-tertiary-rice transition-all"
        onClick={() => selectRef.current?.toggle()}
      >
        <p className="flex items-center justify-center gap-2">
          <IconCalendar className="text-ink-tertiary-500" />
          <span className="diatype-m-bold text-ink-secondary-700"> {m["settings.date"]()}</span>
        </p>
        <Select
          ref={selectRef}
          containerRef={containerRef}
          value={dateFormat}
          onChange={(key) =>
            changeSettings({
              dateFormat: key as AppState["settings"]["dateFormat"],
            })
          }
        >
          <Select.Item value="MM/dd/yyyy">08/29/2025</Select.Item>
          <Select.Item value="dd/MM/yyyy">29/08/2025</Select.Item>
          <Select.Item value="yyyy/MM/dd">2025/08/29</Select.Item>
        </Select>
      </div>
    );
  }

  return (
    <div className="flex items-center justify-between px-2 py-2 rounded-md">
      <p className="flex items-center justify-center gap-2">
        <IconCalendar className="text-ink-tertiary-500" />
        <span className="diatype-m-bold text-ink-secondary-700"> {m["settings.date"]()}</span>
      </p>
      <Select
        value={dateFormat}
        onChange={(key) =>
          changeSettings({
            dateFormat: key as AppState["settings"]["dateFormat"],
          })
        }
      >
        <Select.Item value="MM/dd/yyyy">08/29/2025</Select.Item>
        <Select.Item value="dd/MM/yyyy">29/08/2025</Select.Item>
        <Select.Item value="yyyy/MM/dd">2025/08/29</Select.Item>
      </Select>
    </div>
  );
};

const TimeZoneSection: React.FC = () => {
  const { isMd } = useMediaQuery();
  const { settings, changeSettings } = useApp();
  const { timeZone } = settings;
  const selectRef = useRef<SelectRef>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  if (isMd) {
    return (
      <div
        ref={containerRef}
        className="flex items-center justify-between px-2 py-2 rounded-md cursor-pointer hover:bg-surface-tertiary-rice transition-all"
        onClick={() => selectRef.current?.toggle()}
      >
        <p className="flex items-center justify-center gap-2">
          <IconWorld className="text-ink-tertiary-500" />
          <span className="diatype-m-bold text-ink-secondary-700"> {m["settings.timeZone"]()}</span>
        </p>
        <Select
          ref={selectRef}
          containerRef={containerRef}
          value={timeZone}
          onChange={(key) =>
            changeSettings({
              timeZone: key as AppState["settings"]["timeZone"],
            })
          }
        >
          <Select.Item value="utc">UTC</Select.Item>
          <Select.Item value="local">Local</Select.Item>
        </Select>
      </div>
    );
  }

  return (
    <div className="flex items-center justify-between px-2 py-2 rounded-md">
      <p className="flex items-center justify-center gap-2">
        <IconWorld className="text-ink-tertiary-500" />
        <span className="diatype-m-bold text-ink-secondary-700"> {m["settings.timeZone"]()}</span>
      </p>
      <Select
        value={timeZone}
        onChange={(key) =>
          changeSettings({
            timeZone: key as AppState["settings"]["timeZone"],
          })
        }
      >
        <Select.Item value="utc">UTC</Select.Item>
        <Select.Item value="local">Local</Select.Item>
      </Select>
    </div>
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
