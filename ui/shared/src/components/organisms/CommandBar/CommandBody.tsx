"use client";

import { AnimatePresence, motion } from "framer-motion";

import { AppletCard } from "../../";

import { useStorage } from "@leftcurve/react";
import { Command } from "cmdk";
import type { AppletMetadata } from "../../../types";

interface Props {
  isOpen: boolean;
  isSearching: boolean;
  action: (metadata: AppletMetadata) => void;
  applets: AppletMetadata[];
}

export const CommandBody: React.FC<Props> = ({ isOpen, isSearching, applets, action }) => {
  const popularApplets = applets.filter((applet) => applet.isFeatured);
  const [recentApplets, setRecentApplets] = useStorage<AppletMetadata[]>("applets", {
    initialValue: [],
  });

  const onSelectApplet = (key: string) => {
    const clearPrefix = key.split("_");
    const title = clearPrefix.length > 1 ? clearPrefix[1] : clearPrefix[0];
    const applet = applets.find((applet) => applet.title === title);
    if (!applet) return;

    if (recentApplets.some((recentApplet) => applet.title === recentApplet.title)) {
      setRecentApplets((prevState) => {
        const index = prevState.findIndex((recentApplet) => applet.title === recentApplet.title);
        const recentApplets = Array.from(prevState);
        recentApplets.splice(index, 1);
        recentApplets.unshift(applet);
        return recentApplets;
      });
    } else {
      setRecentApplets((applets) => [applet, ...applets].slice(0, 4));
    }
    action(applet);
  };

  return (
    <AnimatePresence mode="wait">
      {isOpen && (
        <motion.div
          initial={{ opacity: 0, translateY: -100 }}
          animate={{ opacity: 1, translateY: 0 }}
          exit={{ opacity: 0 }}
          className="w-full flex flex-col gap-6 max-w-[calc(100vh-3.5rem)] overflow-scroll scrollbar-none md:p-4"
        >
          <Command.List>
            <Command.Empty>
              <p className="bg-surface-rose-200 rounded-[20px] py-4 px-5 text-black font-semibold text-[1.25rem]">
                No applets found.
              </p>
            </Command.Empty>

            {isSearching ? (
              <Command.Group value="Applets">
                {applets.map((applet) => (
                  <Command.Item
                    key={applet.title}
                    value={applet.title}
                    onSelect={onSelectApplet}
                    className="group"
                  >
                    <AppletCard key={applet.title} metadata={applet} />
                  </Command.Item>
                ))}
              </Command.Group>
            ) : (
              <>
                {recentApplets.length > 0 ? (
                  <Command.Group
                    heading="Recent"
                    className="text-typography-green-300 font-extrabold uppercase tracking-widest"
                  >
                    {recentApplets.map((applet) => (
                      <Command.Item
                        key={`recent_${applet.title}`}
                        value={`recent_${applet.title}`}
                        onSelect={onSelectApplet}
                        className="group normal-case tracking-normal font-normal"
                      >
                        <AppletCard key={applet.title} metadata={applet} />
                      </Command.Item>
                    ))}
                  </Command.Group>
                ) : null}

                <Command.Group
                  heading="Popular"
                  className="text-typography-green-300 font-extrabold uppercase tracking-widest py-2"
                >
                  {popularApplets.map((applet) => (
                    <Command.Item
                      key={`popular_${applet.title}`}
                      value={`popular_${applet.title}`}
                      onSelect={onSelectApplet}
                      className="group normal-case tracking-normal font-normal"
                    >
                      <AppletCard key={applet.title} metadata={applet} />
                    </Command.Item>
                  ))}
                </Command.Group>
              </>
            )}
          </Command.List>
        </motion.div>
      )}
    </AnimatePresence>
  );
};
