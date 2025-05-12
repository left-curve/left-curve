import type React from "react";
import { useSessionKey } from "@left-curve/store";
import { m } from "~/paraglide/messages";
import { useCountdown } from "@left-curve/applets-kit";
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
  const padded = String(value);

  return (
    <div className="flex min-w-[30px] gap-[1px] items-center justify-center">
      <div className="relative h-4 w-full overflow-hidden flex items-center justify-center">
        <AnimatePresence mode="popLayout">
          <motion.div
            key={padded}
            initial={{ y: -20, opacity: 0 }}
            animate={{ y: 0, opacity: 1 }}
            exit={{ y: 20, opacity: 0 }}
            transition={{
              duration: 0.6,
              ease: [0.4, 0, 0.2, 1],
            }}
          >
            {padded}
          </motion.div>
        </AnimatePresence>
      </div>
      <div>{label}</div>
    </div>
  );
}
