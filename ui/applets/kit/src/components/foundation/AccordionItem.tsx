import { AnimatePresence, motion } from "framer-motion";
import type React from "react";
import { type PropsWithChildren, useState } from "react";
import { twMerge } from "../../utils";
import { IconChevronDown } from "./icons/IconChevronDown";

export const AccordionItem: React.FC<
  PropsWithChildren<{ text: string; icon?: React.ReactNode }>
> = ({ children, text, icon }) => {
  const [isOpen, setIsOpen] = useState<boolean>(false);
  return (
    <div className="flex w-full flex-col bg-rice-50 rounded-md p-4 shadow-card-shadow overflow-hidden">
      <div
        className="flex items-center justify-between cursor-pointer"
        onClick={() => setIsOpen(!isOpen)}
      >
        <p className="diatype-m-bold">{text}</p>
        <div className={twMerge("w-6 h-6 transition-all", isOpen ? "rotate-180" : "rotate-0")}>
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
