import { Button, GradientContainer, useWizard } from "@dango/shared";
import type React from "react";
import type { PropsWithChildren } from "react";
import { Link } from "react-router-dom";

export const WizardLoginWrapper: React.FC<PropsWithChildren> = ({ children }) => {
  const { activeStep, data } = useWizard<{ retry: boolean }>();
  const { retry } = data;
  return (
    <div className="flex flex-col items-center justify-center w-full gap-8">
      <GradientContainer className="w-full max-w-2xl *:flex flex-1 flex-col items-center rounded-3xl p-4 md:p-8 gap-8 md:gap-12">
        <p className="font-extrabold text-typography-rose-600 tracking-widest uppercase text-lg">
          {activeStep === 0 && !retry ? "login to portal" : null}
          {activeStep === 0 && retry ? "enter username" : null}
          {activeStep === 1 ? "select credentials " : null}
        </p>
        {children}
      </GradientContainer>
      <Button as={Link} to="/auth/signup" variant="light" className="text-lg">
        Don't have an account?
      </Button>
    </div>
  );
};
