import { forwardRef, useEffect, useState } from "react";

import {
  Button,
  IconButton,
  IconClose,
  IconLink,
  IconNotiStatus,
  Spinner,
  twMerge,
  useApp,
} from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import type { AnyCoin } from "@left-curve/store/types";
import { usePrices, type useBridgeState, type UseSubmitTxReturnType } from "@left-curve/store";

interface BridgeDepositProps {
  coin: AnyCoin;
  config: NonNullable<ReturnType<typeof useBridgeState>["config"]>;
  amount: string;
  deposit: UseSubmitTxReturnType<void, Error, void, unknown>;
  requiresAllowance: boolean;
  allowanceMutation: UseSubmitTxReturnType<void, Error, void, unknown>;
  reset: () => void;
}

type DepositStep = {
  label: string;
  subMsg?: string;
  isLoading?: boolean;
  completedLink?: string;
};

const DepositStepper: React.FC<{ steps: DepositStep[]; currentStep: number }> = ({
  steps,
  currentStep,
}) => {
  return (
    <div className="flex flex-col">
      {steps.map((step, idx) => {
        const isCurrent = idx === currentStep;
        const isLast = idx === steps.length - 1;

        const lastIcon =
          idx < currentStep ? (
            <IconNotiStatus className="text-utility-success-500" />
          ) : step.isLoading ? (
            <Spinner size="xs" color="green" />
          ) : step.completedLink ? (
            <Button
              variant="link"
              className="p-0 h-fit m-0"
              onClick={() => window.open(step.completedLink, "_blank")}
            >
              <IconLink className="w-4 h-4" />
            </Button>
          ) : (
            <p className="diatype-xs-medium text-ink-tertiary-500">
              {m["bridge.deposit.steps.progress"]({
                current: currentStep + 1,
                total: steps.length,
              })}
            </p>
          );

        return (
          <div className="flex flex-col gap-1" key={step.label}>
            <div className="flex items-center justify-between gap-1">
              <div className="flex items-center justify-center w-6">
                <span
                  className={twMerge(
                    "h-2 w-2 rounded-full ",
                    isCurrent ? "bg-brand-red-bean" : "bg-fg-tertiary-400",
                  )}
                />
              </div>

              <div className="flex flex-1 items-start justify-between gap-2 last:pb-0 min-h-6">
                <div className="flex flex-col">
                  <p
                    className={twMerge(
                      "pt-[2px] diatype-sm-bold",
                      isCurrent ? "text-brand-red-bean" : "text-ink-tertiary-500",
                    )}
                  >
                    {step.label}
                  </p>
                </div>
                {idx <= currentStep ? lastIcon : null}
              </div>
            </div>

            <div className="flex gap-2">
              <div className="w-6 flex items-center justify-center">
                {!isLast && <span className="w-[2px] h-4 bg-outline-secondary-gray" />}
              </div>
              {step.subMsg && (
                <p className="diatype-sm-regular text-ink-tertiary-500">{step.subMsg}</p>
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
};

export const BridgeDeposit = forwardRef((props: BridgeDepositProps, _ref) => {
  const { coin, config, amount, requiresAllowance, allowanceMutation, deposit, reset } = props;
  const { hideModal, settings } = useApp();
  const { getPrice } = usePrices();
  const [currentStep, setCurrentStep] = useState(0);
  const [txLink, setTxLink] = useState("");

  const { formatNumberOptions } = settings;

  const amountPrice = getPrice(amount, coin.denom, {
    format: true,
    formatOptions: formatNumberOptions,
  });

  const commonSteps = [
    {
      label: m["bridge.deposit.steps.deposit"](),
      isLoading: currentStep === (requiresAllowance ? 1 : 0),
    },
    {
      label: m["bridge.deposit.steps.depositArrival"](),
      completedLink: txLink,
      subMsg: m["bridge.timeArrival"]({ network: config.chain.id }),
    },
  ];

  const steps: DepositStep[] = requiresAllowance
    ? [
        {
          label: m["bridge.deposit.steps.approve"](),
          isLoading: currentStep === 0,
        },
        ...commonSteps,
      ]
    : commonSteps;

  useEffect(() => {
    (async () => {
      try {
        if (requiresAllowance) {
          await allowanceMutation.mutateAsync();
          setCurrentStep(1);
        }
        const txHash = await deposit.mutateAsync();
        setCurrentStep(requiresAllowance ? 2 : 1);
        setTxLink(`${config.chain.blockExplorers.default.url}/tx/${txHash}`);
        reset();
      } catch (_) {
        hideModal();
      }
    })();
  }, []);

  return (
    <div className="flex flex-col bg-surface-primary-rice md:border border-outline-secondary-gray text-ink-secondary-700 pt-0 md:pt-6 rounded-xl relative p-4 md:p-6 gap-4 w-full md:max-w-[25rem]">
      <IconButton
        className="hidden md:block absolute right-4 top-4"
        variant="link"
        onClick={() => hideModal()}
      >
        <IconClose />
      </IconButton>
      <div className="flex items-center justify-items-center text-center w-full">
        <h2 className="text-ink-primary-900 diatype-lg-medium w-full">
          {m["bridge.deposit.title"]()}
        </h2>
      </div>

      <div className="flex flex-col gap-1">
        <p className="exposure-sm-italic text-ink-disabled-300">{m["bridge.deposit.action"]()}</p>
        <div className="flex items-center justify-between">
          <p className="h3-bold">
            {amount} {coin.symbol}
          </p>
          <img src={coin.logoURI} alt={coin.name} className="h-8 w-8" />
        </div>
        <p className="diatype-sm-regular text-ink-tertiary-500">{amountPrice}</p>
      </div>

      <div>
        <p className="exposure-sm-italic text-ink-disabled-300">{m["bridge.deposit.from"]()}</p>
        <p className="h3-bold">{m["bridge.network"]({ network: config.chain.id })}</p>
      </div>

      <span className="w-full h-[1px] bg-outline-secondary-gray" />

      <p className="exposure-sm-italic text-ink-disabled-300">
        {m["bridge.deposit.continueInYourWallet"]()}
      </p>

      <DepositStepper steps={steps} currentStep={currentStep} />

      {steps.length === currentStep + 1 && (
        <Button onClick={hideModal} fullWidth>
          {m["common.confirm"]()}
        </Button>
      )}
    </div>
  );
});
