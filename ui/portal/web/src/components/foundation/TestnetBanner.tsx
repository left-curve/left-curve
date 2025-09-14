import { IconClose, Marquee } from "@left-curve/applets-kit";

import type React from "react";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useState } from "react";

import { motion } from "framer-motion";

export const TestnetBanner: React.FC = () => {
  const [testBannerVisibility, setTestBannerVisibility] = useState(true);
  const env = import.meta.env.CONFIG_ENVIRONMENT;

  const showBanner = ["dev"].includes(env);
  const text = m["common.advice"]({ environment: m["common.environment"]({ environment: env }) });

  if (!testBannerVisibility || !showBanner) return null;

  return (
    <motion.div
      exit={{ transform: "scaleY(0)", height: 0, opacity: 0 }}
      transition={{ duration: 0.2 }}
      className="h-9 w-full relative top-0  bg-[url('./images/warning-banner.svg')] flex items-center justify-center"
    >
      <Marquee
        className="w-full bg-[#F7CF74] h-fit p-0 uppercase gap-10"
        item={
          <div className="flex gap-10 items-center text-gray-700 diatype-sm-heavy ml-10">
            <span>{text}</span>
            <span>•</span>
          </div>
        }
        speed={50}
      />
      <button
        type="button"
        className="absolute right-3 top-[7px] h-6 w-6 z-10 rounded-full bg-red-bean-50 border border-gray-100 flex items-center justify-center"
        onClick={() => setTestBannerVisibility(false)}
      >
        <IconClose className="text-red-bean-500 w-5 h-5" />
      </button>
    </motion.div>
  );
};
