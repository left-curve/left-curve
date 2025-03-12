import { createFileRoute, useRouter } from "@tanstack/react-router";

import {
  IconButton,
  IconChevronDown,
  Select,
  SelectItem,
  Tab,
  Tabs,
  useMediaQuery,
} from "@left-curve/applets-kit";
import { useAccount } from "@left-curve/store-react";
import { KeyManagment } from "~/components/KeyManagment";
import { m } from "~/paraglide/messages";
import { getLocale, locales, setLocale } from "~/paraglide/runtime";

export const Route = createFileRoute("/(app)/_app/settings")({
  component: SettingsComponent,
});

function SettingsComponent() {
  const isMd = useMediaQuery("md");
  const { history } = useRouter();
  const { isConnected } = useAccount();
  return (
    <div className="w-full md:max-w-[50rem] mx-auto flex flex-col gap-4 p-4 pt-6 mb-16">
      <h2 className="flex gap-2 items-center">
        {isMd ? null : (
          <IconButton variant="link" onClick={() => history.go(-1)}>
            <IconChevronDown className="rotate-90" />
          </IconButton>
        )}
        <span className="h3-bold">{m["settings.title"]()}</span>
      </h2>
      <div className="rounded-xl bg-rice-25 shadow-card-shadow flex flex-col w-full p-1">
        <h3 className="text-lg font-bold px-[10px] py-4">{m["settings.display"]()}</h3>
        <div className="flex items-center justify-between px-[10px] py-2 rounded-md">
          <p>{m["settings.language"]()}</p>
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
        <div className="flex items-center justify-between px-[10px] py-2 rounded-md">
          <p>{m["settings.number"]()}</p>
          <Select defaultSelectedKey="en" label="Number Format">
            <SelectItem key="en">1234.00</SelectItem>
          </Select>
        </div>
        {isConnected ? (
          <div className="flex items-center justify-between px-[10px] py-4 rounded-md hover:bg-rice-50 transition-all cursor-pointer">
            <p>{m["settings.connectToMobile"]()}</p>
          </div>
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
