"use client";

import { AnimatePresence, motion } from "framer-motion";

import { AppLetCard } from "~/components";

import type { AppletMetadata } from "@leftcurve/types";

interface Props {
  isOpen: boolean;
  recentApplets: AppletMetadata[];
  applets: AppletMetadata[];
}

export const CommandBody: React.FC<Props> = ({ isOpen, recentApplets, applets }) => {
  return (
    <AnimatePresence mode="popLayout">
      {isOpen && (
        <motion.div
          initial={{ opacity: 0, translateY: 100 }}
          animate={{ opacity: 1, translateY: 0 }}
          exit={{ opacity: 0, translateY: 100 }}
          className="w-full flex flex-col gap-6 max-w-[calc(100vh-3.5rem)] overflow-scroll scrollbar-none md:p-4"
        >
          {recentApplets.length ? (
            <div className="py-2 flex flex-col gap-3 w-full">
              <h3 className="text-sm font-extrabold text-sand-900 font-diatype-rounded mx-2 tracking-widest">
                RECENT APPS
              </h3>

              <div className="flex w-full flex-col gap-1">
                {recentApplets.map((applet) => (
                  <AppLetCard key={applet.title} metadata={applet} />
                ))}
              </div>
            </div>
          ) : null}
          <div className="py-2 flex flex-col gap-3 w-full">
            <h3 className="text-sm font-extrabold text-sand-900 font-diatype-rounded mx-2 tracking-widest">
              POPULAR APPS
            </h3>
            <div className="flex w-full flex-col gap-1">
              {applets.map((applet) => (
                <AppLetCard key={applet.title} metadata={applet} />
              ))}
            </div>
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
};
