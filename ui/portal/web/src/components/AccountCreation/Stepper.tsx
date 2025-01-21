import { Button, twMerge, useWizard } from "@left-curve/applets-kit";
import { AnimatePresence } from "framer-motion";
import type { PropsWithChildren } from "react";

export const Stepper: React.FC<PropsWithChildren> = ({ children }) => {
  const { activeStep, previousStep } = useWizard();

  return (
    <AnimatePresence mode="wait">
      <div className="flex flex-col items-center justify-center w-full">
        <div className="dango-grid-4x4-M flex flex-col relative items-center justify-between">
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
            <Button
              variant="light"
              onClick={() => previousStep()}
              className="absolute bottom-[-4rem]"
            >
              Back
            </Button>
          ) : null}
        </div>
      </div>
    </AnimatePresence>
  );
};
