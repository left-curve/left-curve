"use client";

import { useWizard } from "@dango/shared";
import type React from "react";
import type { PropsWithChildren } from "react";

export const WizardLoginWrapper: React.FC<PropsWithChildren> = ({ children }) => {
  const { activeStep, data } = useWizard<{ retry: boolean }>();
  const { retry } = data;
  return (
    <div className="w-full max-w-2xl min-h-[20.5rem] flex flex-1 flex-col items-center bg-surface-rose-200 rounded-3xl p-4 md:p-8 gap-8 md:gap-12">
      <p className="font-extrabold text-typography-rose-600 tracking-widest uppercase text-lg">
        {activeStep === 0 && !retry ? "login to portal" : null}
        {activeStep === 0 && retry ? "enter username" : null}
        {activeStep === 1 ? "select credentials " : null}
      </p>
      {children}
    </div>
  );
};
