import { twMerge, useWizard } from "@left-curve/portal-shared";
import type React from "react";
import type { PropsWithChildren } from "react";

export const WizardLoginWrapper: React.FC<PropsWithChildren> = ({ children }) => {
  const { activeStep } = useWizard();

  const isFirstStep = activeStep === 0;
  const isSecondStep = activeStep === 1;

  return (
    <div className="flex flex-col items-center justify-center w-full gap-8">
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
        <div className="flex gap-8">
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
  );
};
