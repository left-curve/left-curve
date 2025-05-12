import { createLazyFileRoute, useRouter } from "@tanstack/react-router";

import {
  IconButton,
  IconChevronDown,
  IconFormatNumber,
  IconInfo,
  IconLanguage,
  IconMobile,
  Select,
  useMediaQuery,
} from "@left-curve/applets-kit";
import { useAccount, useSessionKey } from "@left-curve/store";
import { Modals } from "~/components/modals/RootModal";
import { KeyManagement } from "~/components/settings/KeyManagement";
import { useApp } from "~/hooks/useApp";
import { m } from "~/paraglide/messages";
import { getLocale, locales, setLocale } from "~/paraglide/runtime";
import { useState } from "react";
import { SessionCountdown } from "~/components/settings/SessionCountdown";

export const Route = createLazyFileRoute("/(app)/_app/settings")({
  component: SettingsComponent,
});

function SettingsComponent() {
  const { isMd, isLg } = useMediaQuery();
  const { history } = useRouter();
  const { isConnected } = useAccount();
  const { showModal, changeSettings, settings } = useApp();
  const { formatNumberOptions } = settings;
  const { session } = useSessionKey();

  return (
    <div className="w-full md:max-w-[50rem] mx-auto flex flex-col gap-4 p-4 pt-6 mb-16">
      <h2 className="flex gap-2 items-center">
        {isMd ? null : (
          <IconButton variant="link" onClick={() => history.go(-1)}>
            <IconChevronDown className="rotate-90" />
          </IconButton>
        )}
        <span className="h3-bold text-gray-900">{m["settings.title"]()}</span>
      </h2>
      {session ? (
        <div className="rounded-xl bg-rice-25 shadow-card-shadow flex flex-col w-full px-2 py-4">
          <h3 className="h4-bold text-gray-900 px-2 pb-4">{m["settings.session.title"]()}</h3>
          <div className="flex items-center justify-between pt-2 rounded-md">
            <p className="flex items-center justify-center gap-2 px-2">
              <IconInfo className="text-gray-500" />
              <span className="diatype-m-bold text-gray-700">
                {m["settings.session.remaining"]()}
              </span>
            </p>
            <div>
              <SessionCountdown />
            </div>
          </div>
          <p className="text-gray-500 diatype-sm-regular pl-10 max-w-96 pb-2">
            {m["settings.session.description"]()}
          </p>
        </div>
      ) : null}
      <div className="rounded-xl bg-rice-25 shadow-card-shadow flex flex-col w-full px-2 py-4">
        <h3 className="h4-bold text-gray-900 px-2 pb-4">{m["settings.display"]()}</h3>
        <div className="flex items-center justify-between py-2 rounded-md">
          <p className="flex items-center justify-center gap-2 px-2">
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
        <div className="flex items-center justify-between py-2 rounded-md">
          <p className="flex items-center justify-center gap-2 px-2">
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
        {isConnected && isLg ? (
          <button
            type="button"
            className="flex items-center justify-between py-4 rounded-md hover:bg-rice-50 transition-all cursor-pointer"
            onClick={() => showModal(Modals.QRConnect)}
          >
            <span className="flex items-center justify-center gap-2 px-2">
              <IconMobile className="text-gray-500" />
              <span className="diatype-m-bold text-gray-700">
                {m["settings.connectToMobile"]()}
              </span>
            </span>
          </button>
        ) : null}
        {/*  <div className="flex items-center justify-between px-[10px] py-2 rounded-md">
          <p>Theme</p>
          <Tabs defaultKey="light">
            <Tab title="system">System</Tab>
            <Tab title="light">light</Tab>
          </Tabs>
        </div> */}
      </div>
      <KeyManagement />
    </div>
  );
}
