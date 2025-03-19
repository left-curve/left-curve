import {
  IconButton,
  IconChevronDown,
  ResizerContainer,
  Stepper,
  useMediaQuery,
  useWizard,
} from "@left-curve/applets-kit";
import type React from "react";
import { m } from "~/paraglide/messages";

export const CreateAccountWrapper: React.FC<React.PropsWithChildren> = ({ children }) => {
  const { activeStep } = useWizard();
  const { isMd } = useMediaQuery();

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
            <span className="h2-heavy">{m["accountCreation.title"]()}</span>
          </h2>
          <p className="text-gray-500 diatype-m-medium">
            {m["accountCreation.stepper.description"]({ step: activeStep })}
          </p>
        </div>
        <Stepper
          steps={Array.from({ length: 2 }).map((_, step) =>
            m["accountCreation.stepper.title"]({ step }),
          )}
          activeStep={activeStep}
        />
      </div>
      <ResizerContainer layoutId="create-account" className="w-full max-w-[22.5rem]">
        {children}
      </ResizerContainer>
    </div>
  );
};
