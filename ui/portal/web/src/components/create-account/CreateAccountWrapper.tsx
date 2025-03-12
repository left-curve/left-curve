import {
  IconButton,
  IconChevronDown,
  ResizerContainer,
  Stepper,
  useMediaQuery,
  useWizard,
} from "@left-curve/applets-kit";
import type React from "react";

const steps = ["Account type", "Deposit"];
const subtitle = [
  "Select your account type to interact within the entire ecosystem",
  "Fund your first spot account, you'll be able to create another account later.",
];

export const CreateAccountWrapper: React.FC<React.PropsWithChildren> = ({ children }) => {
  const { activeStep } = useWizard();
  const isMd = useMediaQuery("md");

  return (
    <div className="flex items-center justify-start w-full h-full flex-col md:max-w-[360px] text-center gap-8">
      <div className="flex flex-col gap-4 items-center justify-center w-full">
        <div className="flex flex-col gap-1 items-center justify-center w-full">
          <h2 className="flex gap-2 items-center justify-center w-full relative">
            {isMd ? null : (
              <IconButton
                variant="link"
                onClick={() => history.go(-1)}
                className="absolute left-0 top-0"
              >
                <IconChevronDown className="rotate-90" />
              </IconButton>
            )}
            <span className="h2-heavy">Create New Account</span>
          </h2>
          <p className="text-gray-500 diatype-m-medium">{subtitle[activeStep]}</p>
        </div>
        <Stepper steps={steps} activeStep={activeStep} />
      </div>
      <ResizerContainer layoutId="create-account" className="w-full max-w-[22.5rem]">
        {children}
      </ResizerContainer>
    </div>
  );
};
