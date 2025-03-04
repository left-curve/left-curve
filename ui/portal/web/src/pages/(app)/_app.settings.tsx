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
        <span className="h3-bold">Settings</span>
      </h2>
      <div className="rounded-xl bg-rice-25 shadow-card-shadow flex flex-col w-full p-1">
        <h3 className="text-lg font-bold px-[10px] py-4">Display</h3>
        <div className="flex items-center justify-between px-[10px] py-2 rounded-md">
          <p>Language</p>
          <Select defaultSelectedKey="en" label="Language">
            <SelectItem key="en">English</SelectItem>
          </Select>
        </div>
        <div className="flex items-center justify-between px-[10px] py-2 rounded-md">
          <p>Number Format</p>
          <Select defaultSelectedKey="en" label="Number Format">
            <SelectItem key="en">1234.00</SelectItem>
          </Select>
        </div>
        {isConnected ? (
          <div className="flex items-center justify-between px-[10px] py-4 rounded-md hover:bg-rice-50 transition-all cursor-pointer">
            <p>Connect to mobile</p>
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
