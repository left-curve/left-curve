"use client";

import { motion } from "framer-motion";
import { useEffect, useRef, useState } from "react";

import { useClickAway } from "react-use";
import { twMerge } from "../../../utils";

import { Command } from "cmdk";
import { SearchIcon } from "../../";
import { CommandBody } from "./CommandBody";

import type { AppletMetadata, VisibleRef } from "../../../types";

interface Props {
  applets: AppletMetadata[];
  hamburgerRef: React.RefObject<VisibleRef>;
  isVisible: boolean;
  changeVisibility: (isVisible: boolean) => void;
  action: (applet: AppletMetadata) => void;
}

export const CommandBar: React.FC<Props> = ({ applets, action, isVisible, changeVisibility }) => {
  const menuRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const [searchText, setSearchText] = useState("");

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!isVisible && e.key === "k" && e.ctrlKey) {
        changeVisibility(true);
      } else if (isVisible && e.key === "Escape") {
        changeVisibility(false);
        setSearchText("");
        inputRef.current?.blur();
      } else if (
        !["INPUT", "TEXT"].includes(window.document.activeElement?.nodeName || "") &&
        e.key.length === 1 &&
        /\w/i.test(e.key)
      ) {
        e.preventDefault();
        changeVisibility(true);
        inputRef.current?.focus();
        setSearchText((prev) => prev + e.key);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isVisible]);

  useClickAway(menuRef, (e: any) => {
    if (e.target.getAttribute("hamburger-element") === "true") return;
    changeVisibility(false);
    setSearchText("");
  });

  const triggerAction = (applet: AppletMetadata) => {
    action(applet);
    changeVisibility(false);
    setSearchText("");
    inputRef.current?.blur();
  };

  const handleInteraction = () => {
    changeVisibility(true);
    inputRef.current?.focus();
  };

  return (
    <>
      <div className="relative rounded-2xl w-full lg:max-w-xl">
        <div
          onClick={handleInteraction}
          className={twMerge(
            "bg-surface-green-200 p-1 rounded-2xl flex items-center justify-center w-full lg:max-w-xl z-10 relative group group-hover:bg-surface-green-400 hover:bg-surface-green-400 pr-[3rem] lg:pr-1",
          )}
        >
          <div
            className={twMerge(
              "bg-surface-green-300 flex-1 rounded-xl h-10 transition-all text-typography-green-300 flex items-center justify-center gap-2 px-2 cursor-text text-start",
              { "bg-surface-green-400": isVisible },
            )}
          >
            <SearchIcon className="h-6 w-6" />
            <p className="flex-1 pt-1 truncate w-0">Search for apps and commands</p>
            <p className="px-1.5 py-0.5 items-center font-sans font-normal text-center bg-surface-green-200 rounded-small text-sm shadow-sm hidden lg:inline-flex">
              ⌥ K
            </p>
          </div>
        </div>
        <Command>
          <motion.div
            ref={menuRef}
            className={twMerge(
              "absolute w-full h-full bottom-0 left-0 transition-all rounded-2xl flex flex-col justify-end gap-8 lg:p-1 lg:gap-2 overflow-y-hidden",
              isVisible
                ? "z-50 bg-surface-green-200 w-screen h-screen rounded-none bottom-0 left-0 p-4 lg:w-full lg:h-fit lg:top-0 lg:left-0 lg:rounded-2xl overflow-scroll scrollbar-none"
                : "z-0",
            )}
          >
            <div className="flex order-2 lg:order-1 pr-[3.125rem] lg:pr-0">
              <div
                className={twMerge(
                  "flex items-center gap-2 px-3 lg:px-2 w-full bg-transparent rounded-xl h-10 text-typography-green-300",
                  isVisible ? "bg-surface-green-300 h-10" : "",
                )}
              >
                <SearchIcon className="h-6 w-6" />
                <Command.Input
                  ref={inputRef}
                  onValueChange={setSearchText}
                  value={searchText}
                  placeholder="Search for apps and commands"
                  className="flex-1 bg-transparent text-typography-green-500 placeholder-typography-green-300 pt-1 outline-none"
                />
              </div>
            </div>
            <CommandBody
              isOpen={isVisible}
              applets={applets}
              action={triggerAction}
              isSearching={!!searchText}
            />
          </motion.div>
        </Command>
      </div>
    </>
  );
};
