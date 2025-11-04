import { useControlledState } from "@left-curve/foundation";

import { AnimatePresence, motion } from "framer-motion";
import { IconChevronDownFill } from "./icons/IconChevronDownFill";

import { twMerge } from "@left-curve/foundation";

import type React from "react";
import type { PropsWithChildren } from "react";

type AccordionItemProps = {
  text: string;
  icon?: React.ReactNode;
  classNames?: {
    container?: string;
    text?: string;
    icon?: string;
    menu?: string;
  };
  defaultExpanded?: boolean;
  onChange?: (isOpen: boolean) => void;
  expanded?: boolean;
};

export const AccordionItem: React.FC<PropsWithChildren<AccordionItemProps>> = ({
  children,
  classNames,
  text,
  icon,
  expanded,
  defaultExpanded,
  onChange,
}) => {
  const [isOpen, setIsOpen] = useControlledState<boolean>(expanded, onChange, defaultExpanded);

  return (
    <div
      className={twMerge(
        "flex w-full flex-col bg-surface-tertiary-rice rounded-md p-4 shadow-account-card overflow-hidden",
        classNames?.container,
      )}
    >
      <button
        type="button"
        className="flex items-center justify-between cursor-pointer outline-none w-full"
        onClick={() => setIsOpen(!isOpen)}
      >
        <p className={twMerge("diatype-m-bold", classNames?.text)}>{text}</p>
        <div
          className={twMerge(
            "w-6 h-6 flex items-center justify-center transition-all",
            classNames?.icon,
          )}
        >
          {icon ? (
            icon
          ) : (
            <IconChevronDownFill
              className={twMerge(
                "w-4 h-4 transition-all duration-75",
                isOpen ? "rotate-180" : "rotate-0",
              )}
            />
          )}
        </div>
      </button>

      {isOpen && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.1, ease: "easeInOut" }}
          className={twMerge("overflow-hidden", classNames?.menu)}
        >
          <div className="w-full pt-4">{children}</div>
        </motion.div>
      )}
    </div>
  );
};
