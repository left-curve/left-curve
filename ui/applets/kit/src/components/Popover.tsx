import React, { useId, useImperativeHandle, useRef } from "react";

import { Popover as HPopover, PopoverButton, PopoverPanel } from "@headlessui/react";
import { AnimatePresence, motion } from "framer-motion";
import { ResizerContainer } from "./ResizerContainer";
import { IconChevronDown } from "./icons/IconChevronDown";

import { twMerge } from "#utils/twMerge.js";

export type PopoverRef = {
  close: () => void;
};

type PopoverProps = {
  trigger: React.ReactNode | string;
  menu: React.ReactNode;
  classNames?: {
    base?: string;
    trigger?: string;
    menu?: string;
  };
  showArrow?: boolean;
};

export const Popover = React.forwardRef<PopoverRef, PopoverProps>(
  ({ menu, trigger, classNames, showArrow = true }, ref) => {
    const id = useId();

    const popoverButtonRef = useRef<HTMLButtonElement>(null);

    useImperativeHandle(ref, () => ({
      close: () => {
        popoverButtonRef.current?.click();
      },
    }));

    return (
      <HPopover className={twMerge("relative group w-fit", classNames?.base)}>
        {({ open }) => (
          <>
            <PopoverButton
              ref={popoverButtonRef}
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
  },
);
