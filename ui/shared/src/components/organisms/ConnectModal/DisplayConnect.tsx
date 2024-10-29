"use client";

import { type FormEvent, useState } from "react";
import { useWizard } from "../../../providers";

import { BackArrowIcon, Button, Input, WalletIcon } from "../../";
import { LoadingIndicator } from "./LoadingIndicator";

import { motion } from "framer-motion";
import { twMerge } from "../../../utils";

import type { Connector } from "@leftcurve/types";

export const DisplayConnect: React.FC = () => {
  const { nextStep, previousStep, setData, data } = useWizard<{
    connector: Connector;
    username: string;
  }>();

  const [userName, setUserName] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const { connector } = data;

  const onSubmit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (!userName) return;
    setIsLoading(true);
    setData({ username: userName, connector });
    nextStep();
    setIsLoading(false);
  };

  return (
    <div className="flex items-center justify-start w-full flex-1 flex-col gap-4">
      <Button
        className="z-40 p-1 bg-gray-300 text-white hover:brightness- rounded-full flex items-center justify-center absolute left-4 top-4 h-fit"
        onClick={previousStep}
      >
        <BackArrowIcon className="h-5 w-5" />
      </Button>
      <motion.form
        onSubmit={onSubmit}
        className="flex  items-center justify-start w-full h-full flex-1 flex-col gap-4 max-w-[20rem] p-4 md:p-0"
        key={connector.id}
        initial={{ opacity: 0, translateY: 100 }}
        animate={{ opacity: 1, translateY: 0 }}
        exit={{ opacity: 0, translateY: 100 }}
      >
        <h2 className="text-2xl font-semibold py-4">Connect with {connector.name}</h2>
        <div className="flex items-center justify-center relative">
          {connector.icon ? (
            <img
              className={twMerge(
                "absolute top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 transition-all",
                isLoading ? "h-14 w-14" : "h-16 w-16",
              )}
              src={connector.icon}
              alt={connector.id}
            />
          ) : (
            <WalletIcon
              connectorId={connector.id}
              className={twMerge(
                "absolute top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 transition-all",
                isLoading ? "h-14 w-14" : "h-16 w-16",
              )}
            />
          )}
          <motion.div
            className={twMerge(
              "transition-all relative z-10 scale-100",
              isLoading ? "scale-100" : "scale-0",
            )}
          >
            <LoadingIndicator size="lg" isLoading={isLoading} />
          </motion.div>
        </div>

        <Input
          placeholder="Username"
          onChange={({ target }) => setUserName(target.value)}
          value={userName}
          disabled={isLoading}
        />
        <Button type="submit" className="w-full" disabled={isLoading}>
          Connect
        </Button>
      </motion.form>
    </div>
  );
};
