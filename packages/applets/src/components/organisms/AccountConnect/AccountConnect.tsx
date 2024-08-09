"use client";

import { AnimatePresence, motion } from "framer-motion";
import { useMemo, useState } from "react";
import { Button } from "~/components";

import LoginFlow from "./LoginFlow";
import RegisterFlow from "./SignupFlow";

import type React from "react";

interface Props {
  onFinish?: () => void;
}

export const AccountConnect: React.FC<Props> = ({ onFinish }) => {
  const [selectedOption, setSelectedOption] = useState<"register" | "login" | null>(null);

  const WizardComponent = useMemo(() => {
    if (!selectedOption) return null;
    return selectedOption === "register" ? RegisterFlow : LoginFlow;
  }, [selectedOption]);

  return (
    <div className="flex flex-col items-center">
      <AnimatePresence mode="wait">
        <motion.div className="rounded-xl bg-white p-4 shadow-xl flex flex-col gap-10 min-h-[20rem] md:min-w-[25rem] items-center overflow-hidden md:p-8">
          {WizardComponent ? (
            <WizardComponent changeSelection={setSelectedOption} onFinish={onFinish} />
          ) : (
            <motion.div
              className="flex flex-col gap-10 items-center w-full justify-center"
              initial={{ opacity: 0, translateX: 100 }}
              animate={{ opacity: 1, translateX: 0 }}
              exit={{ opacity: 0, translateX: -100 }}
            >
              <div className="flex flex-col gap-3 items-center justify-center">
                <h1 className="text-xl font-bold font-grotesk">Welcome to Interface</h1>
                <p className="text-xl font-grotesk">to test our new FLOW 😎</p>
              </div>
              <div className="flex flex-col w-full gap-4">
                <Button onClick={() => setSelectedOption("register")}>Sign up</Button>
                <Button onClick={() => setSelectedOption("login")}>Log in</Button>
              </div>
            </motion.div>
          )}
        </motion.div>
      </AnimatePresence>
      <div className="bg-primary-700 text-[0.6rem] text-white rounded-b-lg py-1 px-2 font-bold">
        Powered by -
      </div>
    </div>
  );
};

export const useAccount = () => {};
