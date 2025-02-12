import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";

import {
  Address,
  IconAddCross,
  IconCopy,
  IconTrash,
  Select,
  SelectItem,
  twMerge,
} from "@left-curve/applets-kit";
import { motion } from "framer-motion";

export const Route = createFileRoute("/(app)/_app/settings")({
  component: function SettingsView() {
    const [theme, setTheme] = useState<"light" | "dark">("light");
    return (
      <div className="w-full md:max-w-[50rem] mx-auto flex flex-col gap-4 p-4 pt-6 mb-16">
        <h2 className="text-2xl font-extrabold font-exposure">Settings</h2>
        {/* first element */}
        <div className="rounded-medium bg-rice-25 shadow-card-shadow flex flex-col w-full px-1">
          <h3 className="text-lg font-bold px-[10px] py-4">Display</h3>
          <div className="flex items-center justify-between px-[10px] py-2 rounded-small">
            <p>Language</p>
            <Select defaultSelectedKey="en">
              <SelectItem key="en" textValue="English">
                English
              </SelectItem>
              <SelectItem key="es" textValue="Spanish">
                Spanish
              </SelectItem>
            </Select>
          </div>
          <div className="flex items-center justify-between px-[10px] py-2 rounded-small">
            <p>Number Format</p>
            <Select defaultSelectedKey="en">
              <SelectItem key="en" textValue="English">
                1234.00
              </SelectItem>
              <SelectItem key="es" textValue="Spanish">
                123.4,00
              </SelectItem>
            </Select>
          </div>
          <div className="flex items-center justify-between px-[10px] py-4 rounded-small hover:bg-rice-50 transition-all cursor-pointer">
            <p>Connect to mobile</p>
          </div>
          <div className="flex items-center justify-between px-[10px] py-2 rounded-small">
            <p>Theme</p>
            {/* button components */}
            <motion.ul className="flex text-base relative  items-center w-fit bg-green-bean-200 p-1 rounded-small">
              {Array.from(["System", "light", "moon"]).map((e, i) => {
                const isActive = e === theme;
                const Icon = e === "light" ? null : IconAddCross;

                return (
                  <motion.li
                    className="relative transition-all flex items-center justify-center py-2 px-4 cursor-pointer"
                    key={`navLink-${e}`}
                    onClick={() => setTheme(e as any)}
                  >
                    <p
                      className={twMerge(
                        "italic font-medium font-exposure transition-all relative z-10",
                        isActive ? "text-black" : "text-gray-300",
                      )}
                    >
                      {e}
                    </p>
                    {isActive ? (
                      <motion.div
                        className="w-full h-full rounded-[10px] bg-green-bean-50 absolute bottom-0 left-0 [box-shadow:0px_4px_6px_2px_#1919191F]"
                        layoutId="theme"
                      />
                    ) : null}
                  </motion.li>
                );
              })}
            </motion.ul>
          </div>
        </div>
        {/* second element */}
        <div className="rounded-medium bg-rice-25 shadow-card-shadow flex flex-col w-full p-4 gap-4">
          <div className="flex flex-col md:flex-row gap-4 items-start justify-between">
            <div className="flex flex-col gap-1 max-w-lg">
              <h3 className="text-lg font-bold">Key Management </h3>
              <p className="text-gray-500 text-sm">
                Easily add or delete passkeys to manage access to your account. Each passkey is
                vital for secure logins, ensuring only you can access your account.
              </p>
            </div>
            <button
              type="button"
              className="w-full md:w-fit h-fit [box-shadow:0px_0px_8px_-2px_#FFFFFFA3_inset,_0px_3px_6px_-2px_#FFFFFFA3_inset,_0px_4px_6px_0px_#0000000A,_0px_4px_6px_0px_#0000000A] border-[1px] border-solid [border-image-source:linear-gradient(180deg,_rgba(46,_37,_33,_0.12)_8%,_rgba(46,_37,_33,_0.24)_100%)] bg-red-bean-400 px-6 py-2 rounded-full font-exposure text-red-bean-50 italic font-medium flex gap-2 items-center justify-center"
            >
              <IconAddCross className="w-5 h-5" />
              Add
            </button>
          </div>
          {Array.from({ length: 3 }).map((_, i) => {
            return (
              <div
                key={crypto.randomUUID()}
                className="flex items-center justify-between rounded-2xl border border-rice-200 hover:bg-rice-50 transition-all p-4"
              >
                <div className="flex items-start justify-between w-full gap-8">
                  <div className="min-w-0">
                    <Address
                      className="text-gray-700 font-bold"
                      address="0x6caf21cd9f6D4c6eF7CF32539690B79665abFE09"
                    />

                    <p className="text-gray-500 text-sm">Metamask Wallet</p>
                  </div>
                  <div className="flex gap-1">
                    <IconCopy className="w-5 h-5" />
                    <IconTrash className="w-5 h-5" />
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </div>
    );
  },
});
