"use client";
import type React from "react";

import { type PropsWithChildren, useState } from "react";

import { Button, CloseIcon } from "../../";
import { useConnectors } from "../../../../../../../sdk/packages/dango/src/store/react";
import { useWizard } from "../../../providers";
import { twMerge } from "../../../utils";

import { DisplayConnectors } from "./DisplayConnectors";

import type { Connector } from "@left-curve/types";

export const WrapperConnect: React.FC<PropsWithChildren> = ({ children }) => {
  const [connector, setConnector] = useState<Connector | undefined>();
  const { activeStep, nextStep, reset, setData } = useWizard();
  const connectors = useConnectors();

  const onSelect = (connector: Connector | undefined) => {
    setConnector(connector);
    setData({ connector });
    if (activeStep === 0) nextStep();
  };

  return (
    <div
      className={twMerge(
        "flex flex-col items-start justify-center w-full bg-surface-rose-200 min-h-[35rem] rounded-3xl relative transition-all h-fit",
        activeStep === 2 ? "md:max-w-[33rem] min-h-[25rem]" : "md:max-w-[50rem] min-h-[35rem]",
      )}
    >
      <Button
        className="p-1 bg-gray-300 text-white hover:brightness- rounded-full flex items-center justify-center absolute right-4 top-4 h-fit z-10"
        onClick={reset}
      >
        <CloseIcon className="h-5 w-5" />
      </Button>
      <div className="flex flex-1 w-full flex-col md:flex-row">
        <DisplayConnectors
          connectors={connectors}
          shouldHide={activeStep === 2}
          onSelect={onSelect}
          selected={connector}
        />
        <div className="p-4 md:p-8 flex-1 flex relative transition-all">{children}</div>
      </div>
    </div>
  );
};
