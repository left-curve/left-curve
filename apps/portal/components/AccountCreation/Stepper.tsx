"use client";

import { GradientContainer, twMerge, useWizard } from "../../../../packages/ui/build/index.mjs";
import { AnimatePresence } from "framer-motion";
import type { PropsWithChildren } from "react";

export const Stepper: React.FC<PropsWithChildren> = ({ children }) => {
  const { activeStep, previousStep } = useWizard();

  return (
    <AnimatePresence mode="wait">
      <GradientContainer className="flex flex-col w-full relative items-center justify-center gap-12 max-w-[612px] min-h-[612px] px-12 py-8">
        {children}
        <div className="flex gap-4 text-xs tracking-wider font-semibold">
          <p
            className={twMerge(
              activeStep === 0 ? "text-typography-purple-400" : "text-typography-purple-400/40",
            )}
          >
            1 SELECT
          </p>
          <p
            className={twMerge(
              activeStep === 1 ? "text-typography-purple-400" : "text-typography-purple-400/40",
            )}
          >
            2 TRANSFER
          </p>
        </div>

        {activeStep === 1 ? (
          // biome-ignore lint/a11y/useKeyWithClickEvents: <explanation>
          <div
            className="absolute bottom-[-4rem] text-typography-purple-400 text-lg cursor-pointer"
            onClick={() => previousStep()}
          >
            Back
          </div>
        ) : null}
      </GradientContainer>
    </AnimatePresence>
  );
};
