import type React from "react";
import { useEffect, useState } from "react";

import { intervalToDuration, type Duration } from "date-fns";
import { useSessionKey } from "@left-curve/store";
import { m } from "~/paraglide/messages";
import { Skeleton, twMerge, useCountdown } from "@left-curve/applets-kit";
import { AnimatePresence, motion } from "framer-motion";

export const SessionCountdown: React.FC = () => {
  const { session } = useSessionKey();

  const { hours, minutes, seconds } = useCountdown(Number(session?.sessionInfo.expireAt || 0));

  return (
    <div className="flex gap-1 text-gray-700">
      {hours ? <FlipUnit value={hours} label={m["settings.session.time.hours"]()} /> : null}
      <FlipUnit value={minutes} label={m["settings.session.time.minutes"]()} />
      <FlipUnit value={seconds} label={m["settings.session.time.seconds"]()} />
    </div>
  );
};

type FlipUnitProps = {
  value: number | string;
  label: string;
};

function FlipUnit({ value, label }: FlipUnitProps) {
  const padded1 = String(value).split("")[0];
  const padded2 = String(value).split("")[1];

  return (
    <div className="flex min-w-[30px] gap-[1px] items-center justify-center">
      <AnimatePresence mode="popLayout">
        <motion.div
          key={padded1 + label}
          initial={{ y: -10, opacity: 0 }}
          animate={{ y: 0, opacity: 1 }}
          exit={{ y: 10, opacity: 0 }}
          transition={{ duration: 0.3 }}
        >
          {padded1}
        </motion.div>
      </AnimatePresence>
      <AnimatePresence mode="popLayout">
        <motion.div
          key={padded2 + label}
          initial={{ y: -10, opacity: 0 }}
          animate={{ y: 0, opacity: 1 }}
          exit={{ y: 10, opacity: 0 }}
          transition={{ duration: 0.3 }}
        >
          {padded2}
        </motion.div>
      </AnimatePresence>
      <div>{label}</div>
    </div>
  );
}
