import { IconChevronDown } from "./icons/IconChevronDown";
import { twMerge } from "#utils/twMerge.js";
import { AnimatePresence, motion } from "framer-motion";
import { useEffect, useRef, useState } from "react";
import { useClickAway } from "react-use";

interface Props {
  trigger: React.ReactNode | string;
  menu: React.ReactNode;
  className?: {
    base: string;
    trigger?: string;
    menu?: string;
  };
  showArrow?: boolean;
}

export const Popover: React.FC<Props> = ({ menu, trigger, className, showArrow = true }) => {
  const [open, setOpen] = useState(false);
  const popoverRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);

  useClickAway(popoverRef, () => setOpen(false));

  useEffect(() => {
    if (open && triggerRef.current && panelRef.current) {
      const panel = panelRef.current;

      const top = triggerRef.current.offsetTop + triggerRef.current.offsetHeight + 4;

      panel.style.top = `${top}px`;
    }
  }, [open]);

  return (
    <div ref={popoverRef} className="relative group w-fit">
      <>
        <motion.button
          ref={triggerRef}
          onClick={() => setOpen((prev) => !prev)}
          className={twMerge("flex items-center gap-2 outline-none", className?.trigger)}
        >
          {trigger}
          {showArrow && (
            <IconChevronDown className={twMerge("transition-all w-5 h-5", open && "rotate-180")} />
          )}
        </motion.button>

        {open && (
          <motion.div
            ref={panelRef}
            className={twMerge(
              "left-0 xl:left-1/2 xl:-translate-x-1/2 flex flex-col absolute z-50 bg-rice-25 rounded-lg h-fit p-4 shadow-card-shadow",
              className?.menu,
            )}
          >
            <motion.div layout="size">
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
          </motion.div>
        )}
      </>
    </div>
  );
};
