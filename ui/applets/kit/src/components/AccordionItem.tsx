import { useState } from "react";

import { AnimatePresence, motion } from "framer-motion";
import { IconChevronDown } from "./icons/IconChevronDown";

import { twMerge } from "#utils/twMerge.js";

import type React from "react";
import type { PropsWithChildren } from "react";
import { useControlledState } from "#hooks/useControlledState.js";

type AccordionItemProps = {
  text: string;
  icon?: React.ReactNode;
  classNames?: {
    container?: string;
    text?: string;
    icon?: string;
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
        "flex w-full flex-col bg-rice-50 rounded-md p-4 shadow-account-card overflow-hidden",
        classNames?.container,
      )}
    >
      <div
        className="flex items-center justify-between cursor-pointer"
        onClick={() => setIsOpen(!isOpen)}
      >
        <p className={twMerge("diatype-m-bold", classNames?.text)}>{text}</p>
        <div
          className={twMerge(
            "w-6 h-6 transition-all",
            isOpen ? "rotate-180" : "rotate-0",
            classNames?.icon,
          )}
        >
          {icon ? icon : <IconChevronDown />}
        </div>
      </div>

      <AnimatePresence initial={false}>
        {isOpen && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.3, ease: "easeInOut" }}
            className="overflow-hidden"
          >
            <div className="w-full pt-4">{children}</div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
};
