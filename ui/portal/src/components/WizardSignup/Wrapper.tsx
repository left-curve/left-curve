import { DangoButton, GradientContainer } from "@dango/shared";
import type React from "react";
import type { PropsWithChildren } from "react";
import { Link } from "react-router-dom";

export const WizardSignupWrapper: React.FC<PropsWithChildren> = ({ children }) => {
  return (
    <div className="flex flex-col items-center justify-center w-full gap-8">
      <GradientContainer className="w-full max-w-2xl flex flex-1 flex-col items-center rounded-3xl p-4 md:p-8 gap-8 md:gap-[54px]">
        <p className="font-extrabold text-typography-rose-600 tracking-widest uppercase text-lg">
          sign up
        </p>
        {children}
      </GradientContainer>
      <DangoButton
        type="button"
        as={Link}
        to="/auth/login"
        variant="ghost"
        color="rose"
        className="text-lg italic"
      >
        Already have an account?
      </DangoButton>
    </div>
  );
};
