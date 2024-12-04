import { motion } from "framer-motion";
import type React from "react";
import { useLocation } from "react-router-dom";

import { WizardProvider } from "@dango/shared";
import { AuthWizard } from "~/components/AuthWizard";
import { ConnectorStep } from "~/components/WizardLogin/ConnectorStep";
import { LoginStep } from "~/components/WizardLogin/LoginStep";
import { ConnectStep } from "~/components/WizardSignup/ConnectStep";
import { CredentialStep } from "~/components/WizardSignup/CredentialStep";
import { TransferStep } from "~/components/WizardSignup/TransferStep";

const AuthView: React.FC = () => {
  const location = useLocation();

  const isSignup = location.pathname === "/auth/signup";

  const login = (
    <>
      <LoginStep />
      <ConnectorStep />
    </>
  );

  const singup = (
    <>
      <CredentialStep />
      <ConnectStep />
      <TransferStep />
    </>
  );

  return (
    <main className="flex flex-col min-h-screen w-full h-full bg-surface-off-white-200 overflow-y-auto overflow-x-hidden scrollbar-none items-center justify-center">
      <div className="min-h-full w-full flex-1 flex flex-col justify-center z-10">
        <div className="relative h-[70px] md:h-[112px] px-12 py-7">
          <div className="w-full h-full flex items-center justify-center md:justify-start md:items-start">
            <a href="/">
              <img src="/images/dango.svg" alt="logo" className="h-6 md:h-[31px] object-contain" />
            </a>
          </div>
          <motion.div
            animate={{
              background: isSignup
                ? "linear-gradient(90deg, #D88E96 0%, #C4B7BA 100%)"
                : "linear-gradient(90deg, #C2D0C9 0%, #93A4C8 100%)",
            }}
            className="header w-full absolute h-[75px] md:h-[112px] top-0 left-0 z-[-1]"
          />
        </div>
        <div className="flex flex-1 w-full items-center justify-center p-4">
          <div className="flex flex-1 w-full items-center justify-center p-4 flex-col gap-8">
            <WizardProvider wrapper={<AuthWizard />}>
              {isSignup ? singup.props.children : login.props.children}
            </WizardProvider>
          </div>
        </div>
      </div>
    </main>
  );
};

export default AuthView;
