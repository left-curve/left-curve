"use client";

import { useChainId } from "@leftcurve/react";
import type { Connector, Username } from "@leftcurve/types";
import { sleep } from "@leftcurve/utils";
import { motion } from "framer-motion";
import { useEffect, useState } from "react";
import { BackArrowIcon, Button, WalletIcon } from "~/components";
import { Spinner } from "~/components/atoms/Spinner";
import { useWizard } from "~/providers";
import { twMerge } from "~/utils";

export const DisplayConnection: React.FC = () => {
  const chainId = useChainId();
  const { previousStep, data, done } = useWizard<{
    connector: Connector;
    username: Username;
  }>();

  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { connector, username } = data;

  useEffect(() => {
    connect();
  }, []);

  const connect = async () => {
    try {
      setError(null);
      setIsLoading(true);
      await sleep(1000);
      await connector.connect({ username, chainId });
      done();
    } catch (error) {
      setError(error instanceof Error ? error.message : "something went wrong");
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="flex items-center justify-start w-full flex-1 flex-col gap-4">
      <Button
        className="z-40 p-1 bg-gray-300 text-white hover:brightness- rounded-full flex items-center justify-center absolute left-4 top-4 h-fit"
        onClick={previousStep}
      >
        <BackArrowIcon className="h-5 w-5" />
      </Button>
      <motion.div
        className="flex  items-center justify-start w-full h-full flex-1 flex-col gap-4 max-w-[20rem] p-4 md:p-0"
        key={connector.id}
        initial={{ opacity: 0, translateY: 100 }}
        animate={{ opacity: 1, translateY: 0 }}
        exit={{ opacity: 0, translateY: 100 }}
      >
        <h2 className="text-2xl font-semibold py-4">Connecting with {connector.name}</h2>
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
          <motion.div className="transition-all relative z-10 scale-100">
            <Spinner size="lg" isLoading={isLoading} isError={!!error} />
          </motion.div>
        </div>

        <Button className="w-full" disabled={isLoading} onClick={connect}>
          {error ? "Retry" : "Connecting"}
        </Button>
        {error && <p className="text-red-500">something went wrong</p>}
      </motion.div>
    </div>
  );
};
