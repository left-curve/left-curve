import { Modals, numberMask, Skeleton, useApp, useInputs } from "@left-curve/applets-kit";
import { useAccount, useVaultLiquidityState } from "@left-curve/store";
import { perpsMarginAsset } from "@left-curve/store";

import {
  Button,
  Input,
  Tabs,
  createContext,
  twMerge,
} from "@left-curve/applets-kit";

import { formatNumber } from "@left-curve/dango/utils";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { useEffect, type PropsWithChildren } from "react";
import { MobileTitle } from "../foundation/MobileTitle";

const [VaultLiquidityProvider, useVaultLiquidity] = createContext<{
  state: ReturnType<typeof useVaultLiquidityState>;
  controllers: ReturnType<typeof useInputs>;
}>({
  name: "VaultLiquidityContext",
});

type VaultLiquidityProps = {
  action: "deposit" | "withdraw";
  onChangeAction: (action: "deposit" | "withdraw") => void;
};

const VaultLiquidityContainer: React.FC<PropsWithChildren<VaultLiquidityProps>> = ({
  children,
  action,
  onChangeAction,
}) => {
  const controllers = useInputs({ strategy: "onChange" });

  const state = useVaultLiquidityState({
    action,
    onChangeAction,
    controllers,
  });

  return (
    <VaultLiquidityProvider value={{ state, controllers }}>
      <div
        className={twMerge(
          "w-full mx-auto flex flex-col pt-6 mb-16 gap-8 p-4",
          state.userHasShares ? "md:max-w-[50.1875rem]" : "md:max-w-[25rem]",
        )}
      >
        <MobileTitle title={m["vaultLiquidity.title"]()} />
        {children}
      </div>
    </VaultLiquidityProvider>
  );
};

const VaultLiquidityHeader: React.FC = () => {
  const { state } = useVaultLiquidity();
  const { settings } = useApp();
  const { formatNumberOptions } = settings;
  const { isPaused, userHasShares, vaultState } = state;

  const equity = vaultState?.equity ?? "0";

  return (
    <div className="flex flex-col gap-3">
      {isPaused && (
        <div className="flex items-center gap-2 p-3 bg-primitives-red-light-50 rounded-xl border border-primitives-red-light-200">
          <p className="text-primitives-red-light-600 diatype-sm-medium">
            {m["vaultLiquidity.paused"]()}
          </p>
        </div>
      )}
      <div
        className={twMerge(
          "flex flex-col gap-3 justify-between p-4 rounded-xl shadow-account-card bg-surface-tertiary-rice relative w-full overflow-hidden",
          { "lg:flex-row": userHasShares },
        )}
      >
        <div className="flex gap-2 items-center">
          <img
            src="/images/coins/usd.svg"
            alt="vault"
            className="w-8 h-8 rounded-full"
          />
          <p className="text-ink-secondary-700 h4-bold">
            {m["vaultLiquidity.title"]()}
          </p>
        </div>
        <div
          className={twMerge("flex flex-row justify-between items-center", {
            "lg:justify-center lg:gap-8": userHasShares,
          })}
        >
          <div
            className={twMerge("flex flex-col items-start gap-0", {
              "lg:flex-row lg:gap-1 lg:items-center": userHasShares,
            })}
          >
            <p className="text-ink-tertiary-500 diatype-xs-medium">{m["vaultLiquidity.apy"]()}</p>
            <p className="text-ink-secondary-700 diatype-sm-bold">-</p>
          </div>
          <div
            className={twMerge("flex flex-col items-end gap-0", {
              "lg:flex-row lg:gap-1 lg:items-center": userHasShares,
            })}
          >
            <p className="text-ink-tertiary-500 diatype-xs-medium">{m["vaultLiquidity.tvl"]()}</p>
            <p className="text-ink-secondary-700 diatype-sm-bold">
              {formatNumber(equity, { ...formatNumberOptions, currency: "USD" })}
            </p>
          </div>
        </div>
        <img
          src="/images/characters/hippo.svg"
          alt="dango-hippo"
          className="max-w-[298px] absolute opacity-10 left-[8.75rem] top-0 select-none drag-none"
        />
      </div>
    </div>
  );
};

const VaultLiquidityUserLiquidity: React.FC = () => {
  const { settings } = useApp();
  const { state } = useVaultLiquidity();
  const { formatNumberOptions } = settings;
  const { userHasShares, userVaultShares, userSharesValue, isLoading } = state;

  if (isLoading) return <Skeleton className="h-[9rem] rounded-xl shadow-account-card flex-1" />;

  if (!userHasShares) return null;

  return (
    <div className="flex p-4 flex-col gap-4 rounded-xl bg-surface-secondary-rice shadow-account-card flex-1 h-fit lg:max-w-[373.5px]">
      <div className="flex items-center justify-between">
        <p className="exposure-sm-italic text-ink-tertiary-500">{m["vaultLiquidity.liquidity"]()}</p>
        <p className="h4-bold text-ink-primary-900">
          {formatNumber(userSharesValue, { ...formatNumberOptions, currency: "USD" })}
        </p>
      </div>
      <div className="flex flex-col w-full gap-2">
        <div className="flex items-center justify-between">
          <p className="text-ink-tertiary-500 diatype-m-regular">{m["vaultLiquidity.vaultShares"]()}</p>
          <p className="text-ink-secondary-700 diatype-m-regular">
            {formatNumber(userVaultShares, formatNumberOptions)}
          </p>
        </div>
      </div>
    </div>
  );
};

const VaultLiquidityDeposit: React.FC = () => {
  const { settings, showModal } = useApp();
  const { state, controllers } = useVaultLiquidity();
  const { formatNumberOptions } = settings;
  const { action, isPaused, userMargin, sharesToReceive, deposit } = state;
  const { register, setValue, errors } = controllers;
  const { account } = useAccount();

  if (action !== "deposit") return null;

  const isLoggedIn = !!account;

  return (
    <>
      <div className="flex flex-col gap-2">
        <p className="exposure-sm-italic text-ink-secondary-700">
          {m["vaultLiquidity.toDeposit"]()}
        </p>
        <div className="flex flex-col rounded-xl bg-surface-secondary-rice shadow-account-card">
          <Input
            {...register("depositAmount", {
              validate: (v) => {
                if (Number(v) > Number(userMargin))
                  return m["errors.validations.insufficientFunds"]();
                return true;
              },
              mask: numberMask,
            })}
            hideErrorMessage
            placeholder="0"
            startText="right"
            startContent={
              <div className="flex items-center gap-2 pl-4">
                <img
                  src={perpsMarginAsset.logoURI}
                  alt={perpsMarginAsset.symbol}
                  className="w-8 h-8 rounded-full"
                />
                <p className="text-ink-tertiary-500 diatype-lg-medium">{perpsMarginAsset.symbol}</p>
              </div>
            }
            classNames={{
              inputWrapper:
                "pl-0 py-3 flex-col h-auto gap-[6px] bg-transparent shadow-none",
              input: "!h3-medium",
            }}
            insideBottomComponent={
              isLoggedIn ? (
                <div className="w-full flex justify-between pl-4 h-[22px]">
                  <div className="flex gap-1 items-center justify-center diatype-sm-regular text-ink-tertiary-500">
                    <span>
                      {formatNumber(userMargin, formatNumberOptions)} {perpsMarginAsset.symbol}
                    </span>
                    <Button
                      type="button"
                      variant="tertiary-red"
                      size="xs"
                      className="py-[2px] px-[6px]"
                      onClick={() => setValue("depositAmount", userMargin)}
                    >
                      {m["common.max"]()}
                    </Button>
                  </div>
                </div>
              ) : null
            }
          />
        </div>
        {errors?.depositAmount && (
          <p className="diatype-sm-regular text-status-fail">{errors.depositAmount}</p>
        )}
      </div>
      <div className="flex flex-col gap-2">
        <p className="exposure-sm-italic text-ink-secondary-700">
          {m["vaultLiquidity.toGet"]()}
        </p>
        <div className="flex items-center justify-between p-4 rounded-xl bg-surface-secondary-rice shadow-account-card">
          <p className="text-ink-tertiary-500 diatype-m-regular">{m["vaultLiquidity.vaultShares"]()}</p>
          <p className="text-ink-secondary-700 h3-medium">
            {formatNumber(sharesToReceive, formatNumberOptions)}
          </p>
        </div>
      </div>
      {isLoggedIn ? (
        <Button
          size="md"
          fullWidth
          isDisabled={isPaused}
          isLoading={deposit.isPending}
          onClick={() =>
            showModal(Modals.VaultAddLiquidity, {
              amount: controllers.inputs.depositAmount?.value || "0",
              sharesToReceive,
              confirmAddLiquidity: deposit.mutateAsync,
            })
          }
        >
          {m["common.deposit"]()}
        </Button>
      ) : (
        <Button
          size="md"
          fullWidth
          onClick={() => showModal(Modals.Authenticate)}
        >
          {m["common.signin"]()}
        </Button>
      )}
    </>
  );
};

const VaultLiquidityWithdraw: React.FC = () => {
  const { settings, showModal } = useApp();
  const { state, controllers } = useVaultLiquidity();
  const { formatNumberOptions } = settings;
  const { action, isPaused, userVaultShares, usdToReceive, withdraw } = state;
  const { register, setValue, errors } = controllers;
  const { account } = useAccount();

  if (action !== "withdraw") return null;

  const isLoggedIn = !!account;

  return (
    <>
      <div className="flex flex-col gap-2">
        <p className="exposure-sm-italic text-ink-secondary-700">
          {m["vaultLiquidity.sharesToBurn"]()}
        </p>
        <div className="flex flex-col rounded-xl bg-surface-secondary-rice shadow-account-card">
          <Input
            {...register("withdrawShares", {
              validate: (v) => {
                if (Number(v) > Number(userVaultShares))
                  return m["errors.validations.insufficientFunds"]();
                return true;
              },
              mask: numberMask,
            })}
            hideErrorMessage
            placeholder="0"
            startText="right"
            classNames={{
              inputWrapper:
                "pl-0 py-3 flex-col h-auto gap-[6px] bg-transparent shadow-none",
              input: "!h3-medium",
            }}
            insideBottomComponent={
              isLoggedIn ? (
                <div className="w-full flex justify-between pl-4 h-[22px]">
                  <div className="flex gap-1 items-center justify-center diatype-sm-regular text-ink-tertiary-500">
                    <span>
                      {formatNumber(userVaultShares, formatNumberOptions)} {m["vaultLiquidity.vaultShares"]()}
                    </span>
                    <Button
                      type="button"
                      variant="tertiary-red"
                      size="xs"
                      className="py-[2px] px-[6px]"
                      onClick={() => setValue("withdrawShares", userVaultShares)}
                    >
                      {m["common.max"]()}
                    </Button>
                  </div>
                </div>
              ) : null
            }
          />
        </div>
        {errors?.withdrawShares && (
          <p className="diatype-sm-regular text-status-fail">{errors.withdrawShares}</p>
        )}
      </div>
      <div className="flex flex-col gap-2">
        <p className="exposure-sm-italic text-ink-secondary-700">
          {m["vaultLiquidity.usdToReceive"]()}
        </p>
        <div className="flex items-center justify-between p-4 rounded-xl bg-surface-secondary-rice shadow-account-card">
          <p className="text-ink-tertiary-500 diatype-m-regular">{perpsMarginAsset.symbol}</p>
          <p className="text-ink-secondary-700 h3-medium">
            {formatNumber(usdToReceive, { ...formatNumberOptions, currency: "USD" })}
          </p>
        </div>
      </div>
      {isLoggedIn ? (
        <Button
          size="md"
          fullWidth
          isDisabled={isPaused}
          isLoading={withdraw.isPending}
          onClick={() =>
            showModal(Modals.VaultWithdrawLiquidity, {
              sharesToBurn: controllers.inputs.withdrawShares?.value || "0",
              usdToReceive,
              confirmWithdrawal: withdraw.mutateAsync,
            })
          }
        >
          {m["common.withdraw"]()}
        </Button>
      ) : (
        <Button
          size="md"
          fullWidth
          onClick={() => showModal(Modals.Authenticate)}
        >
          {m["common.signin"]()}
        </Button>
      )}
    </>
  );
};

const VaultLiquidityHeaderTabs: React.FC = () => {
  const { state } = useVaultLiquidity();
  const { action, onChangeAction, userHasShares } = state;

  useEffect(() => {
    if (!userHasShares) {
      onChangeAction("deposit");
    }
  }, [userHasShares]);

  return (
    <Tabs
      layoutId="tabs-vault-deposit-withdraw"
      selectedTab={action}
      keys={userHasShares ? ["deposit", "withdraw"] : ["deposit"]}
      fullWidth
      onTabChange={(tab) => onChangeAction(tab as "deposit" | "withdraw")}
    />
  );
};

export const VaultLiquidity = Object.assign(VaultLiquidityContainer, {
  Header: VaultLiquidityHeader,
  HeaderTabs: VaultLiquidityHeaderTabs,
  UserLiquidity: VaultLiquidityUserLiquidity,
  Deposit: VaultLiquidityDeposit,
  Withdraw: VaultLiquidityWithdraw,
});
