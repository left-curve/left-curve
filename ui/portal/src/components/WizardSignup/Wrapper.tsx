import type React from "react";
import type { PropsWithChildren } from "react";

export const WizardSignupWrapper: React.FC<PropsWithChildren> = ({ children }) => {
  return (
    <div className="w-full max-w-2xl min-h-[20.5rem] flex flex-1 flex-col items-center bg-surface-rose-200 rounded-3xl p-4 md:p-8 gap-8 md:gap-12">
      <p className="font-extrabold text-typography-rose-600 tracking-widest uppercase text-lg">
        sign up to portal
      </p>
      {children}
    </div>
  );
};
