import { useCountdown } from "@left-curve/applets-kit";
import { useSessionKey } from "@left-curve/store";

import { AnimatePresence, motion } from "framer-motion";

import { m } from "~/paraglide/messages";

import type React from "react";

const hoursLabel = m["settings.session.time.hours"]();
const minutesLabel = m["settings.session.time.minutes"]();
const secondsLabel = m["settings.session.time.seconds"]();

export const SessionCountdown: React.FC = () => {
  const { session } = useSessionKey();

  const { hours, minutes, seconds } = useCountdown({
    date: Number(session?.sessionInfo.expireAt || 0),
    showLeadingZeros: true,
  });

  return (
    <div className="flex gap-1 text-secondary-700 px-4 py-3 shadow-account-card rounded-md min-w-[9rem] h-[46px] items-center justify-center">
      {[
        { value: hours, label: hoursLabel },
        { value: minutes, label: minutesLabel },
        { value: seconds, label: secondsLabel },
      ].map((unit: { value: number | string; label: string }) => {
        const { value, label } = unit;
        const padded = String(value);

        if (value === "00" && label === hoursLabel) return null;

        return (
          <div className="flex min-w-[30px] gap-[1px] items-center justify-center " key={label}>
            <div className="relative h-4 w-full overflow-hidden flex items-center justify-center">
              <AnimatePresence mode="popLayout">
                <motion.div
                  key={padded}
                  initial={{ y: -20, opacity: 0 }}
                  animate={{ y: 0, opacity: 1 }}
                  exit={{ y: 20, opacity: 0 }}
                  transition={{
                    duration: 0.6,
                    ease: [1, 0, 0.4, 1],
                  }}
                >
                  {padded}
                </motion.div>
              </AnimatePresence>
            </div>
            <div>{label}</div>
          </div>
        );
      })}
    </div>
  );
};
