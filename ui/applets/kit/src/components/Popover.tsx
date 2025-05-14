import { Popover as HPopover, PopoverButton, PopoverPanel } from "@headlessui/react";
import { IconChevronDown } from "./icons/IconChevronDown";
import { twMerge } from "#utils/twMerge.js";
import { AnimatePresence, motion } from "framer-motion";

interface Props {
  trigger: React.ReactNode | string;
  menu: React.ReactNode;
  classNames?: {
    base?: string;
    trigger?: string;
    menu?: string;
  };
  showArrow?: boolean;
}

export const Popover: React.FC<Props> = ({ menu, trigger, classNames, showArrow = true }) => {
  return (
    <HPopover className={twMerge("relative group w-fit", classNames?.base)}>
      {({ open }) => (
        <>
          <PopoverButton
            className={twMerge("flex items-center gap-2 outline-none", classNames?.trigger)}
          >
            {trigger}
            {showArrow && (
              <IconChevronDown
                className={twMerge("transition-all w-5 h-5", open && "rotate-180")}
              />
            )}
          </PopoverButton>

          <PopoverPanel
            anchor="bottom"
            className={twMerge("flex flex-col absolute z-50 p-2", classNames?.menu)}
          >
            <motion.div
              layout="size"
              className="bg-rice-25 rounded-lg h-fit p-4 shadow-card-shadow"
            >
              <AnimatePresence>
                {open && (
                  <motion.div
                    style={{ overflow: "hidden" }}
                    initial={{ height: 0 }}
                    animate={{ transition: { duration: 0.1 }, height: open ? "auto" : 0 }}
                    exit={{ height: 0 }}
                  >
                    <motion.ul exit={{ opacity: 0 }} transition={{ duration: 0.05 }}>
                      {menu}
                    </motion.ul>
                  </motion.div>
                )}
              </AnimatePresence>
            </motion.div>
          </PopoverPanel>
        </>
      )}
    </HPopover>
  );
};
