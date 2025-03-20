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
import { useNavigate } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";
import { useApp } from "~/hooks/useApp";

import { m } from "~/paraglide/messages";
import { applets } from "../../../applets";

import { Command } from "cmdk";
import { AnimatePresence, motion } from "framer-motion";
import { SearchItem } from "./SearchItem";

const ExportComponent = Object.assign(SearchMenu, {
  Body,
});

export { ExportComponent as SearchMenu };

function SearchMenu() {
  const { isLg } = useMediaQuery();
  const { isSearchBarVisible, setSearchBarVisibility } = useApp();
  const [searchText, setSearchText] = useState("");

  const inputRef = useRef<HTMLInputElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  useClickAway(menuRef, () => {
    if (!isLg) return;
    setSearchBarVisibility(false);
    setSearchText("");
  });

  useEffect(() => {
    if (!isLg) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.metaKey || e.ctrlKey) return;
      if (isSearchBarVisible && e.key === "Escape") {
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
      <ResizerContainer layoutId="search-menu">
        <div
          className={twMerge(
            "flex-col bg-rice-25 rounded-md w-full flex items-center lg:absolute relative lg:-top-5 flex-1 lg:[box-shadow:0px_2px_6px_0px_#C7C2B666] transition-all",
            !isLg && isSearchBarVisible
              ? "h-svh w-screen -left-4 -bottom-4 absolute z-[100] bg-white p-4 gap-4"
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
                    <span>{m["commadBar.placeholder.title"]()}</span>{" "}
                    <TextLoop
                      texts={[
                        m["commadBar.placeholder.transactions"](),
                        m["commadBar.placeholder.apps"](),
                        m["commadBar.placeholder.blocks"](),
                        m["commadBar.placeholder.accounts"](),
                        m["commadBar.placeholder.usernames"](),
                        m["commadBar.placeholder.tokens"](),
                      ]}
                    />
                  </motion.button>
                </AnimatePresence>
              )}
            </div>
          </div>

          <Body isVisible={isSearchBarVisible} hideMenu={hideMenu} />
        </div>
      </ResizerContainer>
    </Command>
  );
}

type SearchMenuBodyProps = {
  isVisible: boolean;
  hideMenu: () => void;
};

export function Body({ isVisible, hideMenu }: SearchMenuBodyProps) {
  const navigate = useNavigate();
  return (
    <AnimatePresence mode="wait" custom={isVisible}>
      {isVisible && (
        <motion.div
          layout
          initial={{ height: 0 }}
          animate={{ height: "auto" }}
          exit={{ height: 0 }}
          transition={{ duration: 0.1 }}
          className="menu w-full overflow-hidden"
        >
          <motion.div
            className="p-1 w-full flex items-center flex-col gap-1"
            variants={{
              hidden: {},
              visible: {
                transition: {
                  delayChildren: 0.1,
                  staggerChildren: 0.05,
                },
              },
            }}
            initial="hidden"
            animate="visible"
          >
            <Command.List className="w-full">
              <Command.Empty>
                <p className="text-gray-500 diatype-m-regular p-2 text-center">
                  {m["commadBar.noResult"]()}
                </p>
              </Command.Empty>
              <Command.Group value="Applets">
                {applets.map((applet) => (
                  <Command.Item
                    key={applet.title}
                    value={applet.title}
                    className="group"
                    onSelect={() => [navigate({ to: applet.path }), hideMenu()]}
                  >
                    <SearchItem.Applet key={applet.title} {...applet} />
                  </Command.Item>
                ))}
              </Command.Group>
              {/*    <Command.Group value="Assets">
                {[].map((token) => (
                  <Command.Item
                    key={token.title}
                    value={token.title}
                    className="group"
                    onSelect={() => [navigate({ to: token.path }), hideMenu()]}
                  >
                    <TokenItem {...token} />
                  </Command.Item>
                ))}
              </Command.Group> */}
            </Command.List>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
