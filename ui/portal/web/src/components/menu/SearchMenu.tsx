import {
  IconButton,
  IconChevronDown,
  IconClose,
  IconSearch,
  ResizerContainer,
  TextLoop,
  twMerge,
  useClickAway,
  useMediaQuery,
} from "@left-curve/applets-kit";
import { Command } from "cmdk";
import { AnimatePresence, motion } from "framer-motion";
import type React from "react";
import { useEffect, useRef, useState } from "react";
import { useApp } from "~/hooks/useApp";
import { SearchMenuBody } from "./SearchMenuBody";

export const SearchMenu: React.FC = () => {
  const isLg = useMediaQuery("lg");
  const { isSearchBarVisible, setSearchBarVisibility } = useApp();
  const [searchText, setSearchText] = useState("");

  const inputRef = useRef<HTMLInputElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  useClickAway(menuRef, (e) => {
    if (!isLg) return;
    setSearchBarVisibility(false);
    setSearchText("");
  });

  useEffect(() => {
    if (!isLg) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!isSearchBarVisible && e.key === "k" && e.ctrlKey) {
        setSearchBarVisibility(true);
      } else if (isSearchBarVisible && e.key === "Escape") {
        setSearchBarVisibility(false);
        setSearchText("");
        inputRef.current?.blur();
      } else if (
        !["INPUT", "TEXT"].includes(window.document.activeElement?.nodeName || "") &&
        e.key.length === 1 &&
        /\w/i.test(e.key)
      ) {
        e.preventDefault();
        setSearchBarVisibility(true);
        inputRef.current?.focus();
        setSearchText((prev) => prev + e.key);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isSearchBarVisible]);

  const hideMenu = () => {
    setSearchBarVisibility(false);
    setSearchText("");
    inputRef.current?.blur();
  };

  return (
    <Command ref={menuRef} className="flex flex-col gap-4 w-full">
      <ResizerContainer>
        <div
          className={twMerge(
            "flex-col bg-rice-25 rounded-md w-full flex items-center lg:absolute relative lg:-top-5 flex-1 lg:[box-shadow:0px_2px_6px_0px_#C7C2B666] transition-all",
            !isLg && isSearchBarVisible
              ? "h-screen w-screen -left-4 -bottom-4 absolute z-[100] bg-white p-4 gap-4"
              : "",
          )}
        >
          <div className="w-full gap-[10px] lg:gap-0 flex items-center">
            {!isLg && isSearchBarVisible ? (
              <IconButton variant="link" onClick={hideMenu}>
                <IconChevronDown className="rotate-90" />
              </IconButton>
            ) : null}
            <div className="flex-col bg-rice-25 [box-shadow:0px_2px_6px_0px_#C7C2B666] lg:shadow-none rounded-md w-full flex items-center">
              <motion.div className="w-full flex items-center gap-2 px-3 py-2 rounded-md">
                <IconSearch className="w-5 h-5 text-gray-500" />
                <Command.Input
                  ref={inputRef}
                  onValueChange={setSearchText}
                  value={searchText}
                  className="bg-rice-25 pt-[4px] w-full outline-none focus:outline-none placeholder:text-gray-500"
                />

                {!isLg && searchText ? (
                  <IconClose className="w-6 h-6 text-gray-500" onClick={() => setSearchText("")} />
                ) : null}
              </motion.div>
              {!isSearchBarVisible && (
                <AnimatePresence mode="wait" custom={isSearchBarVisible}>
                  <motion.button
                    type="button"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{
                      opacity: 0,
                      transition: { duration: 0.2 },
                    }}
                    transition={{ duration: 1 }}
                    className="flex absolute w-full h-full bg-transparent left-0 rounded-md cursor-text gap-1 items-center pl-9 pt-1 diatype-m-regular"
                    onClick={() => setSearchBarVisibility(!isSearchBarVisible)}
                  >
                    <span>Search for</span> <TextLoop texts={["transaction", "apps"]} />
                  </motion.button>
                </AnimatePresence>
              )}
            </div>
          </div>

          <SearchMenuBody isVisible={isSearchBarVisible} hideMenu={hideMenu} />
        </div>
      </ResizerContainer>
    </Command>
  );
};
