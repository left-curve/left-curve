"use client";

import { motion } from "framer-motion";
import { useRef, useState } from "react";

import { useClickAway } from "react-use";
import { twMerge } from "../../../utils";

import { Button } from "../../";
import { CloseIcon, SearchIcon } from "../../";
import { CommandBody } from "./CommandBody";

import type { AppletMetadata } from "../../../types";

interface Props {
  applets: {
    popular: AppletMetadata[];
    all: AppletMetadata[];
  };
  action: (applet: AppletMetadata) => void;
}

export const CommandBar: React.FC<Props> = ({ applets, action }) => {
  const [isOpen, setIsOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const [searchText, setSearchText] = useState("");

  useClickAway(menuRef, (e) => {
    setIsOpen(false);
    setSearchText("");
  });

  const handleInteraction = () => {
    setIsOpen(true);
    inputRef.current?.focus();
  };

  return (
    <>
      <div className="xl:col-span-2  min-w-full md:min-w-0 flex-1 order-3 md:order-2 flex items-center justify-center relative">
        <div className="relative z-0 rounded-2xl w-full lg:max-w-xl">
          <div
            onClick={handleInteraction}
            className="bg-gray-50 p-1 rounded-2xl border border-white flex items-center justify-center w-full lg:max-w-xl z-10 relative"
          >
            <div className="bg-gray-200/50 flex-1 rounded-xl h-9 hover:bg-gray-300/50 transition-all text-gray-400 flex items-center gap-2 px-2 cursor-text text-start">
              <SearchIcon className="h-6 w-6" />
              <p className="flex-1 pt-1">Search for apps and commands</p>
              <p>/</p>
            </div>
          </div>
          <motion.div
            ref={menuRef}
            className={twMerge(
              "absolute w-full h-full top-0 left-0 transition-all bg-gray-50 rounded-2xl flex flex-col gap-8 md:p-1 md:gap-2",
              isOpen
                ? "z-50 bg-gray-100 w-screen h-screen rounded-none top-[-72px] left-[-1rem] p-4 md:w-full md:h-fit md:top-0 md:left-0 md:rounded-2xl overflow-scroll scrollbar-none"
                : "z-0",
            )}
          >
            <div className="flex">
              <div
                className={twMerge(
                  "flex items-center gap-2 px-3 md:px-2 w-full bg-transparent rounded-xl h-9 text-gray-400",
                  isOpen ? "bg-gray-300/50 h-10" : "",
                )}
              >
                <SearchIcon className="h-6 w-6" />
                <input
                  ref={inputRef}
                  value={searchText}
                  onChange={(e) => setSearchText(e.target.value)}
                  placeholder="Search for apps and commands"
                  className="flex-1 bg-transparent text-gray-800 placeholder-gray-400 pt-1 outline-none"
                />
                <p>/</p>
              </div>
              <Button
                className={twMerge(
                  "h-10 rounded-xl transition-all overflow-hidden md:hidden",
                  isOpen ? "w-10 px-2 ml-3" : "w-0 p-0",
                )}
                onClick={() => setIsOpen(false)}
                color="danger"
              >
                <CloseIcon className="h-6 w-6" />
              </Button>
            </div>
            <CommandBody
              isOpen={isOpen}
              applets={applets}
              action={action}
              searchText={searchText}
            />
          </motion.div>
        </div>
      </div>
    </>
  );
};
