"use client";

import { motion } from "framer-motion";
import { useEffect, useRef, useState } from "react";

import { useClickAway } from "react-use";
import { twMerge } from "../../../utils";

import { Command } from "cmdk";
import { Button } from "../../";
import { CloseIcon, SearchIcon } from "../../";
import { CommandBody } from "./CommandBody";

import type { AppletMetadata } from "../../../types";

interface Props {
  applets: AppletMetadata[];
  action: (applet: AppletMetadata) => void;
}

export const CommandBar: React.FC<Props> = ({ applets, action }) => {
  const [isOpen, setIsOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const [searchText, setSearchText] = useState("");

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!isOpen && e.key === "k" && e.ctrlKey) {
        setIsOpen(true);
      } else if (isOpen && e.key === "Escape") {
        setIsOpen(false);
        setSearchText("");
        inputRef.current?.blur();
      } else if (
        !["INPUT", "TEXT"].includes(window.document.activeElement?.nodeName || "") &&
        e.key.length === 1 &&
        /\w/i.test(e.key)
      ) {
        e.preventDefault();
        setIsOpen(true);
        inputRef.current?.focus();
        setSearchText((prev) => prev + e.key);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isOpen]);

  useClickAway(menuRef, (e) => {
    setIsOpen(false);
    setSearchText("");
  });

  const triggerAction = (applet: AppletMetadata) => {
    action(applet);
    setIsOpen(false);
    setSearchText("");
    inputRef.current?.blur();
  };

  const handleInteraction = () => {
    setIsOpen(true);
    inputRef.current?.focus();
  };

  return (
    <>
      <div className="xl:col-span-2  min-w-full md:min-w-0 flex-1 order-3 md:order-2 flex items-center justify-center relative">
        <div className="relative rounded-2xl w-full lg:max-w-xl">
          <div
            onClick={handleInteraction}
            className="bg-surface-green-200 p-1 rounded-2xl flex items-center justify-center w-full lg:max-w-xl z-10 relative group group-hover:bg-surface-green-400 hover:bg-surface-green-400"
          >
            <div
              className={twMerge(
                "bg-surface-green-300 flex-1 rounded-xl h-9 transition-all text-typography-green-300 flex items-center gap-2 px-2 cursor-text text-start",
                { "bg-surface-green-400": isOpen },
              )}
            >
              <SearchIcon className="h-6 w-6" />
              <p className="flex-1 pt-1">Search for apps and commands</p>
              <p className="px-1.5 py-0.5 inline-flex items-center font-sans font-normal text-center bg-surface-green-200 rounded-small text-sm shadow-sm">
                ⌥ K
              </p>
            </div>
          </div>
          <Command>
            <motion.div
              ref={menuRef}
              className={twMerge(
                "absolute w-full h-full top-0 left-0 transition-all rounded-2xl flex flex-col gap-8 md:p-1 md:gap-2 overflow-y-hidden",
                isOpen
                  ? "z-50 bg-surface-green-200 w-screen h-screen rounded-none top-[-72px] left-[-1rem] p-4 md:w-full md:h-fit md:top-0 md:left-0 md:rounded-2xl overflow-scroll scrollbar-none"
                  : "z-0",
              )}
            >
              <div className="flex">
                <div
                  className={twMerge(
                    "flex items-center gap-2 px-3 md:px-2 w-full bg-transparent rounded-xl h-9 text-typography-green-300",
                    isOpen ? "bg-surface-green-300 h-10" : "",
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
                <Button
                  className={twMerge(
                    "overflow-hidden md:hidden",
                    isOpen ? "w-10 px-2 ml-3" : "w-0 p-0 opacity-0",
                  )}
                  onClick={() => setIsOpen(false)}
                  isIconOnly
                  radius="lg"
                >
                  <CloseIcon className="h-6 w-6" />
                </Button>
              </div>
              <CommandBody
                isOpen={isOpen}
                applets={applets}
                action={triggerAction}
                isSearching={!!searchText}
              />
            </motion.div>
          </Command>
        </div>
      </div>
    </>
  );
};
