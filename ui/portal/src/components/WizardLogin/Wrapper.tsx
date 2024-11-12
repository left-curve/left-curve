import { Button, twMerge, useWizard } from "@dango/shared";
import type React from "react";
import type { PropsWithChildren } from "react";
import { Link } from "react-router-dom";

export const WizardLoginWrapper: React.FC<PropsWithChildren> = ({ children }) => {
  const { activeStep, data } = useWizard<{ retry: boolean }>();
  const { retry } = data;

  return (
    <div className="flex flex-col items-center justify-center w-full gap-8">
      <div
        className={twMerge("flex flex-col items-center", {
          "dango-grid-landscape-fat-l": activeStep === 0 || retry,
          "dango-grid-square-l": activeStep !== 0 && !retry,
        })}
      >
        <div className="flex flex-col gap-4 items-center">
          <p className="font-extrabold text-typography-black-200 tracking-widest uppercase text-lg">
            {activeStep === 0 && !retry ? "login to portal" : null}
            {activeStep === 0 && retry ? "enter username" : null}
            {activeStep === 1 ? "login " : null}
          </p>
          <p className="text-typography-black-100 text-lg text-center">
            {activeStep === 0 && !retry ? "Enter your username" : null}
            {activeStep === 0 && retry
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
        </div>
      </div>
      <Button as={Link} to="/auth/signup" variant="light" className="text-lg">
        Don't have an account?
      </Button>
    </div>
  );
};
