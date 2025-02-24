import {
  Birdo,
  Button,
  IconLeft,
  ResizerContainer,
  Stepper,
  useWizard,
} from "@left-curve/applets-kit";
import { useAccount } from "@left-curve/store-react";
import { useNavigate } from "@tanstack/react-router";
import type React from "react";
import { useEffect } from "react";

const steps = ["Authenticate", "Username"];

const headers = [
  {
    title: "Create Your Account",
    subtitle: "Choose a log in credential. You can add or remove credentials afterwards.",
  },
  /* {
    title: "Fund Your Account",
    subtitle: "Fund your first spot account, you’ll be able to create another account later.",
  }, */
  {
    title: "Select a Username",
    subtitle: "Your username will be public on-chain and cannot be changed afterwards.",
  },
];

export const SignupWrapper: React.FC<React.PropsWithChildren> = ({ children }) => {
  const { activeStep, previousStep } = useWizard();
  const { isConnected } = useAccount();
  const navigate = useNavigate();
  const { title, subtitle } = headers[activeStep];

  useEffect(() => {
    if (isConnected) navigate({ to: "/" });
  }, []);

  return (
    <div className="h-screen w-screen flex items-center justify-center">
      <div className="flex items-center justify-center flex-1">
        <ResizerContainer className="w-full max-w-[22.5rem]">
          <div className="flex items-center justify-center gap-8 px-4 lg:px-0 flex-col">
            {/* Header */}
            <div className="flex flex-col gap-7 items-center justify-center">
              <img src="./images/dango.svg" alt="dango-logo" className="h-[24px]" />
              <div className="flex flex-col gap-3 items-center justify-center text-center">
                <h1 className="h2-heavy">{title}</h1>
                <p className="text-gray-500 diatype-m-medium">{subtitle}</p>
              </div>
              <Stepper steps={steps} activeStep={activeStep} />
            </div>
            {/* Body */}
            {children}
            {/* Footer */}
            {activeStep === 0 ? (
              <div className="flex items-center gap-1">
                <p>Already have an account? </p>
                <Button variant="link" autoFocus={false} onClick={() => navigate({ to: "/login" })}>
                  Log in
                </Button>
              </div>
            ) : (
              <div className="flex items-center flex-col">
                <Button variant="link" className="text-red-bean-400 hover:text-red-bean-600">
                  Do this later
                </Button>
                <Button
                  size="sm"
                  variant="link"
                  className="flex justify-center items-center"
                  onClick={() => previousStep()}
                >
                  <IconLeft className="w-[22px] h-[22px]" />
                  <span>Back</span>
                </Button>
              </div>
            )}
          </div>
        </ResizerContainer>
      </div>
      <div className="h-full min-w-[720px] w-[720px] hidden xl:flex bg-[url('./images/frame-rounded.svg')] bg-no-repeat bg-cover bg-center items-center justify-center gap-12 flex-col">
        <Birdo className="max-w-[450px] h-auto" />
        <div className="flex flex-col items-center justify-center gap-1">
          <h3 className="exposure-h3-italic">Welcome home</h3>
          <p className="text-gray-500 text-md">The good old days are here to stay.</p>
        </div>
      </div>
    </div>
  );
};
