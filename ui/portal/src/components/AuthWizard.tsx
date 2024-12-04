import { motion } from "framer-motion";
import type { PropsWithChildren } from "react";
import { useLocation, useNavigate } from "react-router-dom";

import { Button, useMeasure, useWizard } from "@dango/shared";
import { WizardLoginWrapper } from "./WizardLogin/Wrapper";
import { WizardSignupWrapper } from "./WizardSignup/Wrapper";

export const AuthWizard: React.FC<PropsWithChildren> = ({ children }) => {
  const [containerRef, { height }] = useMeasure<HTMLDivElement>();
  const navigate = useNavigate();
  const location = useLocation();
  const { activeStep, previousStep } = useWizard();

  const isSignup = location.pathname === "/auth/signup";
  const Wrapper = isSignup ? WizardSignupWrapper : WizardLoginWrapper;

  return (
    <div className="flex flex-1 h-full w-full flex-col justify-center items-center gap-4 md:gap-8">
      <motion.div
        key={`${location.pathname}_${activeStep}`}
        transition={{ duration: 0.5 }}
        initial={{ height: height ? height : "auto" }}
        animate={{ height: "auto" }}
        className="overflow-hidden w-full bg-surface-rose-100 rounded-3xl max-w-2xl shadow-md"
      >
        <div ref={containerRef}>
          <Wrapper>{children}</Wrapper>
        </div>
      </motion.div>
      <Button
        type="button"
        variant="light"
        color="rose"
        className="italic"
        onClick={() =>
          activeStep ? previousStep() : navigate(isSignup ? "/auth/login" : "/auth/signup")
        }
      >
        {activeStep ? "Back" : isSignup ? "Already have an account?" : "Don't have an account?"}
      </Button>
    </div>
  );
};
