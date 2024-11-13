import { Button, twMerge, useWizard } from "@dango/shared";
import type React from "react";
import type { PropsWithChildren } from "react";
import { Link } from "react-router-dom";

export const WizardSignupWrapper: React.FC<PropsWithChildren> = ({ children }) => {
  const { activeStep } = useWizard();
  return (
    <div className="flex flex-col items-center justify-center w-full gap-8">
      <div
        className={twMerge("flex flex-col items-center", {
          "dango-grid-landscape-fat-l": activeStep === 0,
          "dango-grid-square-l": activeStep !== 0,
        })}
      >
        <div className="flex flex-col gap-4 items-center">
          <p className="font-extrabold text-typography-black-200 tracking-widest uppercase text-lg">
            {[0, 1].includes(activeStep) ? "signup" : null}
            {activeStep === 2 ? "new spot account" : null}
          </p>
          <p className="text-typography-black-100 text-lg text-center">
            {activeStep === 0
              ? "Choose your username. It will be public onchain and cannot be changed afterwards."
              : null}
            {activeStep === 1
              ? "Choose a sign-in credential. You can add or remove credentials afterwards."
              : null}
            {activeStep === 2
              ? "Fund your first spot account with USDC from other existing wallets of yours."
              : null}
          </p>
        </div>
        <div className="flex flex-1 justify-center items-center w-full">{children}</div>
        <div className="flex gap-4">
          <p
            className={twMerge(
              "text-[10px] font-semibold tracking-[0.125rem]",
              activeStep === 0 ? "text-typography-purple-400" : "text-typography-purple-300",
            )}
          >
            1 USERNAME
          </p>
          <p
            className={twMerge(
              "text-[10px] font-semibold tracking-[0.125rem]",
              activeStep === 1 ? "text-typography-purple-400" : "text-typography-purple-300",
            )}
          >
            2 CREDENTIAL
          </p>
          <p
            className={twMerge(
              "text-[10px] font-semibold tracking-[0.125rem]",
              activeStep === 2 ? "text-typography-purple-400" : "text-typography-purple-300",
            )}
          >
            3 DEPOSIT
          </p>
        </div>
      </div>
      <Button
        type="button"
        as={Link}
        to="/auth/login"
        variant="light"
        color="rose"
        className="text-lg italic"
      >
        Already have an account?
      </Button>
    </div>
  );
};
