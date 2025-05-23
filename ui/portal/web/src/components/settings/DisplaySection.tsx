import { useMediaQuery } from "@left-curve/applets-kit";
import { useAccount } from "@left-curve/store";
import { useApp } from "~/hooks/useApp";

import {
  IconFormatNumber,
  IconLanguage,
  IconMobile,
  Select,
  Tab,
  Tabs,
} from "@left-curve/applets-kit";
import { Modals } from "../modals/RootModal";

import { m } from "~/paraglide/messages";
import { getLocale, locales, setLocale } from "~/paraglide/runtime";

import type { PropsWithChildren } from "react";
import type React from "react";

const Container: React.FC<PropsWithChildren> = ({ children }) => {
  return (
    <div className="rounded-xl bg-rice-25 shadow-card-shadow flex flex-col w-full px-2 pt-2 pb-4 gap-4">
      <h3 className="h4-bold text-gray-900 px-2 pt-2">{m["settings.display"]()}</h3>
      {children}
    </div>
  );
};

const LanguageSection: React.FC = () => {
  return (
    <div className="flex items-center justify-between px-2 rounded-md">
      <p className="flex items-center justify-center gap-2">
        <IconLanguage className="text-gray-500" />
        <span className="diatype-m-bold text-gray-700">{m["settings.language"]()}</span>
      </p>
      <Select value={getLocale()} onChange={(key) => setLocale(key as "en")}>
        {locales.map((locale) => (
          <Select.Item key={locale} value={locale}>
            {m["settings.languages"]({ language: locale })}
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
        <IconFormatNumber className="text-gray-500" />
        <span className="diatype-m-bold text-gray-700"> {m["settings.number"]()}</span>
      </p>

      <Select
        defaultValue={formatNumberOptions?.mask.toString() || "1"}
        onChange={(key) => [
          changeSettings({
            formatNumberOptions: { ...formatNumberOptions, mask: Number(key) as 1 },
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

const ConnectMobileSection: React.FC = () => {
  const { showModal } = useApp();
  const { isConnected } = useAccount();
  const { isLg } = useMediaQuery();

  if (!isConnected && !isLg) return null;

  return (
    <div className="flex w-full pr-2">
      <button
        type="button"
        className="flex items-center justify-between pl-2 py-4 rounded-md hover:bg-rice-50 transition-all cursor-pointer w-full"
        onClick={() => showModal(Modals.QRConnect)}
      >
        <span className="flex items-center justify-center gap-2">
          <IconMobile className="text-gray-500" />
          <span className="diatype-m-bold text-gray-700">{m["settings.connectToMobile"]()}</span>
        </span>
      </button>
    </div>
  );
};

const ThemeSection: React.FC = () => {
  return (
    <div className="flex items-center justify-between px-[10px] py-2 rounded-md">
      <p>Theme</p>
      <Tabs defaultKey="light" layoutId="theme">
        <Tab title="system">System</Tab>
        <Tab title="light">light</Tab>
      </Tabs>
    </div>
  );
};

export const DisplaySection = Object.assign(Container, {
  Language: LanguageSection,
  FormatNumber: FormatNumberSection,
  ConnectMobile: ConnectMobileSection,
  Theme: ThemeSection,
});
