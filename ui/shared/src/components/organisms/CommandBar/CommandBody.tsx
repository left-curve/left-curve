"use client";

import { AnimatePresence, motion } from "framer-motion";

import { AppletCard } from "../../";

import { useMemo } from "react";
import { CommandBodyPreview } from "./CommandBodyPreview";

import type { AppletMetadata } from "../../../types";

interface Props {
  isOpen: boolean;
  action: (applet: AppletMetadata) => void;
  searchText?: string;
  applets: AppletMetadata[];
}

export const CommandBody: React.FC<Props> = ({ isOpen, applets, searchText, action }) => {
  const popularApplets = applets.filter((applet) => applet.isFeatured);

  const filteredApplets = useMemo(() => {
    if (!searchText) return applets;

    const search = searchText.toLowerCase();

    return applets.filter((applet) => {
      return (
        applet.title.toLowerCase().includes(search) ||
        applet.description.toLowerCase().includes(search)
      );
    });
  }, [searchText, applets]);

  return (
    <AnimatePresence mode="popLayout">
      {isOpen && (
        <motion.div
          initial={{ opacity: 0, translateY: -100 }}
          animate={{ opacity: 1, translateY: 0 }}
          exit={{ opacity: 0, translateY: -100 }}
          className="w-full flex flex-col gap-6 max-w-[calc(100vh-3.5rem)] overflow-scroll scrollbar-none md:p-4"
        >
          {searchText ? (
            <>
              <h3 className="text-sm font-extrabold text-sand-900 font-diatype-rounded mx-2 tracking-widest">
                FOUND APPS
              </h3>
              <div className="flex w-full flex-col gap-1">
                {filteredApplets.map((applet) => (
                  <AppletCard key={applet.title} metadata={applet} onClick={action} />
                ))}
              </div>
            </>
          ) : (
            <CommandBodyPreview popularApplets={popularApplets} action={action} />
          )}
        </motion.div>
      )}
    </AnimatePresence>
  );
};
