import { twMerge, useClickAway, useMediaQuery } from "@left-curve/applets-kit";
import { useNavigate } from "@tanstack/react-router";
import { useEffect, useRef } from "react";
import { useApp } from "~/hooks/useApp";
import { useSearchBar } from "~/hooks/useSearchBar";

import { m } from "~/paraglide/messages";

import {
  IconButton,
  IconChevronDown,
  IconClose,
  IconSearch,
  ResizerContainer,
  Spinner,
  TextLoop,
} from "@left-curve/applets-kit";
import { Command } from "cmdk";
import { AnimatePresence, motion } from "framer-motion";
import { SearchItem } from "./SearchItem";

import type React from "react";
import type { SearchBarResult } from "~/hooks/useSearchBar";

const SearchMenu: React.FC = () => {
  const { isLg } = useMediaQuery();
  const { isSearchBarVisible, setSearchBarVisibility } = useApp();
  const { searchText, setSearchText, isLoading, searchResult, isRefetching } = useSearchBar();

  const inputRef = useRef<HTMLInputElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  useClickAway(menuRef, () => {
    if (!isLg) return;
    setSearchBarVisibility(false);
    setSearchText("");
  });

  useEffect(() => {
    if (!isLg) return;
    const handleKeyDown = async (e: KeyboardEvent) => {
      if (!isSearchBarVisible && e.key === "k" && e.metaKey) {
        inputRef.current?.focus();
        setSearchBarVisibility(true);
      } else if (e.metaKey || e.ctrlKey) {
        return;
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

  const openMenu = () => {
    setSearchBarVisibility(true);
    inputRef.current?.focus();
  };

  const hideMenu = () => {
    setSearchBarVisibility(false);
    setSearchText("");
    inputRef.current?.blur();
  };

  return (
    <Command ref={menuRef} className="flex flex-col gap-4 w-full" shouldFilter={false}>
      <ResizerContainer layoutId="search-menu">
        <div
          className={twMerge(
            "flex-col bg-surface-secondary-rice rounded-md h-[44px] lg:h-auto w-full flex items-center lg:absolute relative lg:top-[-22px] flex-1 lg:shadow-account-card transition-all duration-300",
            !isLg && isSearchBarVisible
              ? "h-svh w-screen -left-4 -bottom-4 absolute z-[100] bg-surface-primary-rice p-4 gap-4"
              : "",
          )}
        >
          <div className="w-full gap-[10px] lg:gap-0 flex items-center">
            {!isLg && isSearchBarVisible ? (
              <IconButton variant="link" onClick={hideMenu}>
                <IconChevronDown className="rotate-90" />
              </IconButton>
            ) : null}
            <div className="flex-col bg-surface-secondary-rice shadow-account-card lg:shadow-none rounded-md w-full flex items-center">
              <motion.div className="w-full flex items-center gap-2 px-3 py-2 rounded-md">
                <IconSearch className="w-5 h-5 text-tertiary-500" />
                <Command.Input
                  ref={inputRef}
                  onValueChange={setSearchText}
                  value={searchText}
                  className="bg-surface-secondary-rice pt-[4px] w-full outline-none focus:outline-none placeholder:text-tertiary-500"
                />

                {!isLg && searchText ? (
                  <IconClose
                    className="w-6 h-6 text-tertiary-500"
                    onClick={() => setSearchText("")}
                  />
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
                    onClick={() => (isSearchBarVisible ? hideMenu() : openMenu())}
                  >
                    <span>{m["searchBar.placeholder.title"]()}</span>{" "}
                    <TextLoop
                      texts={[
                        m["searchBar.placeholder.transactions"](),
                        m["searchBar.placeholder.apps"](),
                        m["searchBar.placeholder.blocks"](),
                        m["searchBar.placeholder.accounts"](),
                        m["searchBar.placeholder.usernames"](),
                        m["searchBar.placeholder.tokens"](),
                      ]}
                    />
                  </motion.button>
                </AnimatePresence>
              )}
            </div>
          </div>

          <Body
            isVisible={isSearchBarVisible}
            hideMenu={hideMenu}
            searchResult={searchResult}
            isLoading={isLoading || isRefetching}
          />
        </div>
      </ResizerContainer>
    </Command>
  );
};

type SearchMenuBodyProps = {
  isVisible: boolean;
  hideMenu: () => void;
  searchResult: SearchBarResult;
  isLoading: boolean;
};

const Body: React.FC<SearchMenuBodyProps> = ({ isVisible, hideMenu, searchResult, isLoading }) => {
  const navigate = useNavigate();
  const { applets, block, txs, account, contract } = searchResult;

  return (
    <AnimatePresence mode="wait" custom={isVisible}>
      {isVisible && (
        <motion.div
          layout
          initial={{ height: 0 }}
          animate={{ height: "auto" }}
          exit={{ height: 0 }}
          transition={{ duration: 0.1 }}
          className="menu w-full overflow-hidden md:max-h-[25.15rem] lg:overflow-y-auto scrollbar-thin scrollbar-thumb-rice-100 scrollbar-track-transparent"
        >
          <motion.div
            className="lg:p-1 w-full flex items-center flex-col gap-1"
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
                {isLoading ? (
                  <div className="flex items-center justify-center w-full p-2">
                    <Spinner color="pink" size="lg" />
                  </div>
                ) : (
                  <p className="text-tertiary-500 diatype-m-regular p-2 text-center">
                    {m["searchBar.noResult"]()}
                  </p>
                )}
              </Command.Empty>
              {applets.length ? (
                <Command.Group heading="Applets">
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
              ) : null}
              {block ? (
                <Command.Group heading="Block">
                  <Command.Item
                    key={block.hash}
                    value={block.hash}
                    className="group"
                    onSelect={() => [navigate({ to: `/block/${block.blockHeight}` }), hideMenu()]}
                  >
                    <SearchItem.Block height={block.blockHeight} hash={block.hash} />
                  </Command.Item>
                </Command.Group>
              ) : null}
              {txs.length
                ? txs.map((tx) => (
                    <Command.Group heading="Transactions" key={tx.hash}>
                      <Command.Item
                        key={tx.hash}
                        value={tx.hash}
                        className="group"
                        onSelect={() => [navigate({ to: `/tx/${tx.hash}` }), hideMenu()]}
                      >
                        <SearchItem.Transaction height={tx.blockHeight} hash={tx.hash} />
                      </Command.Item>
                    </Command.Group>
                  ))
                : null}
              {account ? (
                <Command.Group heading="Accounts">
                  <Command.Item
                    key={account.address}
                    value={account.address}
                    className="group"
                    onSelect={() => [navigate({ to: `/account/${account.address}` }), hideMenu()]}
                  >
                    <SearchItem.Account account={account} />
                  </Command.Item>
                </Command.Group>
              ) : null}
              {contract ? (
                <Command.Group heading="Contracts">
                  <Command.Item
                    key={contract.address}
                    value={contract.address}
                    className="group"
                    onSelect={() => [navigate({ to: `/contract/${contract.address}` }), hideMenu()]}
                  >
                    <SearchItem.Contract contract={contract} />
                  </Command.Item>
                </Command.Group>
              ) : null}
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
};

const ExportComponent = Object.assign(SearchMenu, {
  Body,
});

export { ExportComponent as SearchMenu };
