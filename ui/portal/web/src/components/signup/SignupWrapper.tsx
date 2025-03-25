import {
  Birdo,
  Button,
  Carousel,
  IconLeft,
  ResizerContainer,
  Stepper,
  useWizard,
} from "@left-curve/applets-kit";
import { useAccount } from "@left-curve/store";
import { Link, useNavigate } from "@tanstack/react-router";
import type React from "react";
import { useEffect } from "react";
import { m } from "~/paraglide/messages";

export const SignupWrapper: React.FC<React.PropsWithChildren> = ({ children }) => {
  const { activeStep, previousStep } = useWizard();
  const { isConnected } = useAccount();
  const navigate = useNavigate();

  useEffect(() => {
    if (isConnected) navigate({ to: "/" });
  }, []);

  return (
    <div className="h-screen w-screen flex items-center justify-center">
      <div className="flex items-center justify-center flex-1">
        <ResizerContainer layoutId="signup" className="w-full max-w-[22.5rem]">
          <div className="flex items-center justify-center gap-8 px-4 lg:px-0 flex-col">
            {/* Header */}
            <div className="flex flex-col gap-7 items-center justify-center">
              <img
                src="./favicon.svg"
                alt="dango-logo"
                className="h-12 rounded-full shadow-btn-shadow-gradient"
              />
              <div className="flex flex-col gap-3 items-center justify-center text-center">
                <h1 className="h2-heavy">{m["signup.stepper.title"]({ step: activeStep })}</h1>
                <p className="text-gray-500 diatype-m-medium">
                  {m["signup.stepper.description"]({ step: activeStep })}
                </p>
              </div>
              <Stepper
                steps={Array.from({ length: 2 }).map((_, step) =>
                  m["signup.stepper.steps"]({ step }),
                )}
                activeStep={activeStep}
              />
            </div>
            {/* Body */}
            {children}
            {/* Footer */}
            {activeStep === 0 ? (
              <div className="flex items-center gap-1">
                <p>{m["signup.alreadyHaveAccount"]()}</p>
                <Button variant="link" autoFocus={false} onClick={() => navigate({ to: "/login" })}>
                  {m["common.signin"]()}
                </Button>
              </div>
            ) : (
              <div className="flex items-center flex-col">
                <Button
                  as={Link}
                  to="/"
                  variant="link"
                  className="text-red-bean-400 hover:text-red-bean-600"
                >
                  {m["signup.doThisLater"]()}
                </Button>
                <Button
                  size="sm"
                  variant="link"
                  className="flex justify-center items-center"
                  onClick={() => previousStep()}
                >
                  <IconLeft className="w-[22px] h-[22px]" />
                  <span>{m["common.back"]()}</span>
                </Button>
              </div>
            )}
          </div>
        </ResizerContainer>
      </div>
      <div className="custom-width h-full min-w-[720px] w-[720px] hidden xl:flex bg-[url('./images/frame-rounded.svg')] bg-no-repeat bg-cover bg-center items-center justify-center">
        <Carousel className="gap-2 sm:gap-4 xl:gap-6 w-full flex-1 py-4">
          <div className="flex flex-col items-center justify-center gap-8 text-center px-4 xl:px-0  flex-1">
            <img
              src="/images/characters/birdo.svg"
              alt="birdo"
              className="w-full max-w-[14rem] sm:max-w-[22rem] h-auto object-contain"
              draggable={false}
            />
            <div className="flex flex-col items-center justify-center gap-1 max-w-full lg:max-w-[25rem]">
              <h3 className="exposure-h3-italic">Welcome home</h3>
              <p className="text-gray-500 text-md">The good old days are here to stay.</p>
            </div>
          </div>
          <div className="flex flex-col items-center justify-center gap-8 text-center px-4 xl:px-0  flex-1">
            <img
              src="/images/characters/birdo.svg"
              alt="birdo"
              className="w-full max-w-[14rem] sm:max-w-[22rem] h-auto object-contain"
              draggable={false}
            />
            <div className="flex flex-col items-center justify-center gap-1 max-w-full lg:max-w-[25rem]">
              <h3 className="exposure-h3-italic">Use Dango</h3>
              <p className="text-gray-500 text-md">
                Lorem ipsum dolor sit amet, consectetur adipiscing elit.
              </p>
            </div>
          </div>
          <div className="flex flex-col items-center justify-center gap-8 text-center px-4 xl:px-0  flex-1">
            <img
              src="/images/characters/birdo.svg"
              alt="birdo"
              className="w-full max-w-[14rem] sm:max-w-[22rem] h-auto object-contain"
              draggable={false}
            />
            <div className="flex flex-col items-center justify-center gap-1 max-w-full lg:max-w-[25rem]">
              <h3 className="exposure-h3-italic">How to use it</h3>
              <p className="text-gray-500 text-md">Fusce purus justo, lobortis aliquet orci.</p>
            </div>
          </div>
        </Carousel>
      </div>
    </div>
  );
};
