import { Popover as HPopover, PopoverButton, PopoverPanel } from "@headlessui/react";
import { IconChevronDown } from "./icons/IconChevronDown";
import { twMerge } from "#utils/twMerge.js";
import { AnimatePresence, motion } from "framer-motion";
import { useId } from "react";
import { ResizerContainer } from "./ResizerContainer";

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
  const id = useId();
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
            className={twMerge("flex flex-col absolute z-50 p-2 scrollbar-none")}
          >
            <ResizerContainer
              layoutId={`popover-menu-${id}`}
              className={twMerge(
                "bg-rice-25 rounded-xl h-fit p-4 shadow-account-card",
                classNames?.menu,
              )}
            >
              <AnimatePresence>
                {open && (
                  <motion.div
                    initial={{ height: 0, opacity: 0 }}
                    animate={{ height: "auto", opacity: 1 }}
                    exit={{ height: 0, opacity: 0 }}
                    transition={{ duration: 0.3, ease: "easeInOut" }}
                  >
                    {menu}
                  </motion.div>
                )}
              </AnimatePresence>
            </ResizerContainer>
          </PopoverPanel>
        </>
      )}
    </HPopover>
  );
};
