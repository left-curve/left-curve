import React from "react";
import { twMerge } from "#utils/twMerge.js";

interface Props {
  steps: unknown[];
  activeStep: number;
}

export const Stepper: React.FC<Props> = ({ steps, activeStep }) => {
  return (
    <div className="flex w-[calc(100%-4rem)] items-center pb-9 min-h-[4rem] diatype-sm-bold">
      {steps.map((step, i) => {
        const isActive = i === activeStep;
        return (
          <React.Fragment key={`${step}`}>
            <div className="relative">
              <StepIcon index={i} active={activeStep} />
              <p
                className={twMerge(
                  "absolute min-w-fit block top-9 left-1/2 -translate-x-1/2 transition-all text-nowrap",
                  {
                    "text-red-bean-600": isActive,
                  },
                )}
              >
                {step as string}
              </p>
            </div>
            {i < steps.length - 1 && (
              <span
                className={twMerge(
                  "w-full h-[2px]  transition-all",
                  i < activeStep ? "bg-red-bean-500" : "bg-gray-100",
                )}
              />
            )}
          </React.Fragment>
        );
      })}
    </div>
  );
};

interface StepProps {
  index: number;
  active: number;
}

const StepIcon: React.FC<StepProps> = ({ index, active }) => {
  if (index < active) {
    return (
      <svg
        xmlns="http://www.w3.org/2000/svg"
        width="25"
        height="24"
        fill="none"
        viewBox="0 0 25 24"
      >
        <path
          fill="#F57589"
          d="M.666 12c0-6.627 5.373-12 12-12s12 5.373 12 12-5.373 12-12 12-12-5.373-12-12"
        />
        <path
          fill="#fff"
          fillRule="evenodd"
          d="m17.762 7.39-7.16 6.91-1.9-2.03c-.35-.33-.9-.35-1.3-.07-.39.29-.5.8-.26 1.21l2.25 3.66c.22.34.6.55 1.03.55.41 0 .8-.21 1.02-.55.36-.47 7.23-8.66 7.23-8.66.9-.92-.19-1.73-.91-1.03z"
          clipRule="evenodd"
        />
      </svg>
    );
  }

  return (
    <div
      className={twMerge(
        "transition-all rounded-full flex items-center justify-center relative z-20",
        "before:content-[''] before:absolute before:z-10 before:bg-bg-primary-rice before:border before:border-red-bean-400 before:w-7 before:h-7 before:rounded-full before:bg-reb-bean-400 before:transition-all",
        "",
        active === index
          ? "border-white bg-red-bean-400 before:scale-1 w-7 h-7 border-4"
          : "border-gray-200 before:scale-0 bg-gray-25 w-6 h-6 border-2",
      )}
    >
      <div
        className={twMerge("w-2 h-2 rounded-full", active === index ? "bg-white" : "bg-gray-300")}
      />
    </div>
  );
};
