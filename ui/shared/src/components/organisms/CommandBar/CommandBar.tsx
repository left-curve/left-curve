"use client";

import { motion } from "framer-motion";
import { useEffect, useRef, useState } from "react";

import { useClickAway } from "react-use";
import { twMerge } from "../../../utils";

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
      if (isOpen && e.key === "Escape") {
        setIsOpen(false);
        setSearchText("");
        inputRef.current?.blur();
        return;
      }
      if (
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
            className="bg-surface-green-200 p-1 rounded-2xl flex items-center justify-center w-full lg:max-w-xl z-10 relative group group-hover:bg-surface-green-300 hover:bg-surface-green-300"
          >
            <div className="bg-surface-green-300 flex-1 rounded-xl h-9  group-hover:bg-surface-green-200 hover:bg-surface-green-200 transition-all text-typography-green-300 flex items-center gap-2 px-2 cursor-text text-start">
              <SearchIcon className="h-6 w-6" />
              <p className="flex-1 pt-1">Search for apps and commands</p>
              <p>/</p>
            </div>
          </div>
          <motion.div
            ref={menuRef}
            className={twMerge(
              "absolute w-full h-full top-0 left-0 transition-all bg-surface-green-300 rounded-2xl flex flex-col gap-8 md:p-1 md:gap-2",
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
                <input
                  ref={inputRef}
                  value={searchText}
                  onChange={(e) => setSearchText(e.target.value)}
                  placeholder="Search for apps and commands"
                  className="flex-1 bg-transparent text-typography-green-4 placeholder-typography-green-300 pt-1 outline-none"
                />
                <p>/</p>
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
              searchText={searchText}
            />
          </motion.div>
        </div>
      </div>
    </>
  );
};
