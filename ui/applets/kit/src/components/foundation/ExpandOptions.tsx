import { Children, useState } from "react";

import { twMerge } from "../../utils";

import { AnimatePresence, motion } from "framer-motion";
import { IconChevronDown } from "./icons/IconChevronDown";

import type React from "react";
import type { PropsWithChildren } from "react";

type ExpandOptionsProps = {
  showOptionText: string;
  className?: string;
};

export const ExpandOptions: React.FC<PropsWithChildren<ExpandOptionsProps>> = ({
  children,
  showOptionText,
  className,
}) => {
  const [isOptionExpanded, setOptionExpanded] = useState(false);

  const options = Children.toArray(children);

  return (
    <div className={className}>
      <div className="flex items-center justify-center text-gray-500">
        <span className="flex-1 h-[1px] bg-gray-100" />
        <div
          className="flex items-center justify-center gap-1 px-2 cursor-pointer"
          onClick={() => setOptionExpanded(!isOptionExpanded)}
        >
          <p>{showOptionText}</p>
          <IconChevronDown
            className={twMerge(
              "w-4 h-4 transition-all duration-300",
              isOptionExpanded ? "rotate-180" : "rotate-0",
            )}
          />
        </div>
        <span className="flex-1 h-[1px] bg-gray-100" />
      </div>
      <motion.div layout className="overflow-hidden">
        <AnimatePresence>
          {isOptionExpanded && (
            <motion.div
              key="options"
              initial={{ opacity: 0, height: 0, paddingBottom: 0 }}
              animate={{ opacity: 1, height: "auto", paddingBottom: "1rem" }}
              exit={{ opacity: 0, height: 0, paddingBottom: 0 }}
              transition={{ duration: 0.2 }}
              className="flex flex-col"
            >
              <motion.div
                className="flex flex-col gap-3 py-4"
                variants={{
                  hidden: {},
                  visible: {
                    transition: {
                      delayChildren: 0.1,
                      staggerChildren: 0.1,
                    },
                  },
                }}
                initial="hidden"
                animate="visible"
              >
                {options.map((option) => (
                  <motion.div
                    key={crypto.randomUUID()}
                    variants={{
                      hidden: { opacity: 0, y: -30 },
                      visible: { opacity: 1, y: 0 },
                    }}
                  >
                    {option}
                  </motion.div>
                ))}
              </motion.div>
            </motion.div>
          )}
        </AnimatePresence>
      </motion.div>
    </div>
  );
};
