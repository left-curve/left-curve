import { createFileRoute, useRouter } from "@tanstack/react-router";

import {
  IconButton,
  IconChevronDown,
  IconFormatNumber,
  IconLanguage,
  IconMobile,
  Select,
  SelectItem,
  Tab,
  Tabs,
  useMediaQuery,
} from "@left-curve/applets-kit";
import { useAccount } from "@left-curve/store-react";
import { KeyManagment } from "~/components/KeyManagment";
import { Modals } from "~/components/Modal";
import { useApp } from "~/hooks/useApp";
import { m } from "~/paraglide/messages";
import { getLocale, locales, setLocale } from "~/paraglide/runtime";

export const Route = createFileRoute("/(app)/_app/settings")({
  component: SettingsComponent,
});

function SettingsComponent() {
  const { isMd, isLg } = useMediaQuery();
  const { history } = useRouter();
  const { isConnected } = useAccount();
  const { showModal, setFormatNumberOptions, formatNumberOptions } = useApp();
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
      <div className="rounded-xl bg-rice-25 shadow-card-shadow flex flex-col w-full px-2 py-4">
        <h3 className="h4-bold text-gray-900 px-2 pb-4">{m["settings.display"]()}</h3>
        <div className="flex items-center justify-between py-2 rounded-md">
          <p className="flex items-center justify-center gap-2 px-2">
            <IconLanguage className="text-gray-500" />
            <span className="diatype-m-bold text-gray-700">{m["settings.language"]()}</span>
          </p>
          <Select
            defaultSelectedKey={getLocale()}
            label="Language"
            onSelectionChange={(key) => setLocale(key.toString() as any)}
          >
            {locales.map((locale) => (
              <SelectItem key={locale}>{m["settings.languages"]({ language: locale })}</SelectItem>
            ))}
          </Select>
        </div>
        <div className="flex items-center justify-between py-2 rounded-md">
          <p className="flex items-center justify-center gap-2 px-2">
            <IconFormatNumber className="text-gray-500" />
            <span className="diatype-m-bold text-gray-700"> {m["settings.number"]()}</span>
          </p>

          <Select
            defaultSelectedKey={formatNumberOptions.language}
            label="Format Number Options"
            onSelectionChange={(key) => [
              setFormatNumberOptions((prev) => ({
                ...prev,
                language: key.toString(),
                useGrouping: key !== "de-DE",
              })),
            ]}
          >
            <SelectItem key="en-US">1,234.00</SelectItem>
            <SelectItem key="es-ES">1.234,00</SelectItem>
            <SelectItem key="de-DE">1234,00</SelectItem>
            <SelectItem key="fr-FR">1 234,00</SelectItem>
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
      <KeyManagment />
    </div>
  );
}
