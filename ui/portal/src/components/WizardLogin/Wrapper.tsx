import { Button, twMerge, useMeasure, useWizard } from "@dango/shared";
import { motion } from "framer-motion";
import type React from "react";
import type { PropsWithChildren } from "react";
import { useNavigate } from "react-router-dom";

export const WizardLoginWrapper: React.FC<PropsWithChildren> = ({ children }) => {
  const { activeStep, previousStep } = useWizard();
  const [containerRef, { height }] = useMeasure<HTMLDivElement>();
  const navigate = useNavigate();

  const isFirstStep = activeStep === 0;
  const isSecondStep = activeStep === 1;

  return (
    <div className="flex flex-col items-center justify-center w-full gap-8">
      <motion.div
        key={activeStep}
        transition={{ duration: 0.5 }}
        initial={{ height }}
        animate={{ height: "auto" }}
        className="overflow-hidden w-full bg-surface-rose-100 rounded-3xl max-w-2xl shadow-md"
      >
        <div ref={containerRef}>
          <div className="flex flex-col items-center px-8 py-6 gap-12 w-full">
            <div className="flex flex-col gap-8 md:gap-10 w-full">
              <div className="flex flex-col gap-4 items-center">
                <p className="text-typography-rose-700 typography-headline-m uppercase">log in</p>
                {isSecondStep ? (
                  <p className="text-typography-rose-600 text-lg text-center">
                    Choose any of the credentials that have been associated with your username.
                  </p>
                ) : null}
              </div>
              <div className="flex flex-1 justify-center items-center w-full">{children}</div>
            </div>
            <div className="flex gap-4">
              <p
                className={twMerge(
                  "text-[10px] font-semibold tracking-[0.125rem]",
                  isFirstStep ? "text-typography-purple-400" : "text-typography-purple-300",
                )}
              >
                1 USERNAME
              </p>
              <p
                className={twMerge(
                  "text-[10px] font-semibold tracking-[0.125rem]",
                  isSecondStep ? "text-typography-purple-400" : "text-typography-purple-300",
                )}
              >
                2 CREDENTIAL
              </p>
            </div>
          </div>
        </div>
      </motion.div>
      <Button
        variant="light"
        onClick={() => (isFirstStep ? navigate("/auth/signup") : previousStep())}
      >
        {isFirstStep ? "Don't have an account?" : "Back"}
      </Button>
    </div>
  );
};
