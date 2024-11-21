import { Button, twMerge, useWizard } from "@dango/shared";
import type React from "react";
import type { PropsWithChildren } from "react";
import { useNavigate } from "react-router-dom";

export const WizardLoginWrapper: React.FC<PropsWithChildren> = ({ children }) => {
  const { activeStep, data, previousStep } = useWizard<{ retry: boolean }>();
  const navigate = useNavigate();
  const { retry } = data;

  const isFirstStep = activeStep === 0;
  const isSecondStep = activeStep === 1;

  return (
    <div className="flex flex-col items-center justify-center w-full gap-8">
      <div
        className={twMerge("flex flex-col items-center", {
          "dango-grid-3x4-L": isFirstStep || retry,
          "dango-grid-4x4-M": !isFirstStep && !retry,
        })}
      >
        <div className="flex flex-col gap-4 items-center">
          <p className="font-extrabold text-typography-black-200 tracking-widest uppercase text-lg">
            {isFirstStep && !retry ? "login to portal" : null}
            {isFirstStep && retry ? "enter username" : null}
            {isSecondStep ? "login " : null}
          </p>
          <p className="text-typography-black-100 text-lg text-center">
            {isFirstStep && !retry ? "Enter your username" : null}
            {isFirstStep && retry
              ? "The username connected does not match the on-chain record"
              : null}
            {activeStep === 1
              ? "Choose any of the credentials that have been associated with your username."
              : null}
          </p>
        </div>
        <div className="flex flex-1 justify-center items-center w-full">{children}</div>
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
      <Button
        variant="light"
        className="text-lg"
        onClick={() => (isFirstStep ? navigate("/auth/signup") : previousStep())}
      >
        {isFirstStep ? "Already have an account?" : "Back"}
      </Button>
    </div>
  );
};
