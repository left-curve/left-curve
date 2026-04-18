import {
  Button,
  Input,
  Modals,
  RangeWithButtons,
  Skeleton,
  Tabs,
  WarningContainer,
  createContext,
  numberMask,
  useApp,
  useInputs,
} from "@left-curve/applets-kit";
import { perpsMarginAsset, useAccount, useVaultLiquidityState } from "@left-curve/store";
import { formatNumber } from "@left-curve/dango/utils";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useEffect, useState, type PropsWithChildren } from "react";
import { MobileTitle } from "../foundation/MobileTitle";
import { UserWithdrawals } from "./UserWithdrawals";

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
  const state = useVaultLiquidityState({ action, onChangeAction, controllers });
  const { account } = useAccount();
  const isLoggedIn = !!account;

  return (
    <VaultLiquidityProvider value={{ state, controllers }}>
      <div
        className={`w-full mx-auto flex flex-col md:flex-row pt-6 mb-16 gap-4 px-4 md:px-0 ${
          isLoggedIn ? "md:max-w-[50rem]" : "md:max-w-[25rem]"
        }`}
      >
        <div className="flex flex-col gap-4 w-full md:max-w-[25rem]">
          <MobileTitle title={m["vaultLiquidity.title"]()} />
          {children}
        </div>
        {isLoggedIn && (
          <div className="flex flex-col gap-4 w-full md:max-w-[24rem]">
            <UserPosition />
          </div>
        )}
      </div>
    </VaultLiquidityProvider>
  );
};

const VaultLiquidityHeader: React.FC = () => {
  const { state } = useVaultLiquidity();
  const { settings } = useApp();
  const { formatNumberOptions } = settings;
  const { isPaused, isTvlCapReached, vaultState, isLoading } = state;
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
      <div className="flex flex-col gap-3 p-4 rounded-xl shadow-account-card bg-surface-tertiary-rice relative overflow-hidden">
        <div className="flex gap-2 items-center">
          <img src="/images/coins/usd.svg" alt="vault" className="w-8 h-8 rounded-full" />
          <p className="text-ink-secondary-700 h4-bold">{m["vaultLiquidity.title"]()}</p>
        </div>
        <div className="flex flex-row justify-between items-center">
          <div className="flex flex-col items-start">
            <p className="text-ink-tertiary-500 diatype-xs-medium">{m["vaultLiquidity.apy"]()}</p>
            <p className="text-ink-secondary-700 diatype-sm-bold">-</p>
          </div>
          <div className="flex flex-col items-end">
            <p className="text-ink-tertiary-500 diatype-xs-medium">{m["vaultLiquidity.tvl"]()}</p>
            {isLoading ? (
              <Skeleton className="w-16 h-5" />
            ) : (
              <div className="flex items-center gap-2">
                <p className="text-ink-secondary-700 diatype-sm-bold">
                  {formatNumber(equity, { ...formatNumberOptions, currency: "USD" })}
                </p>
                {isTvlCapReached && (
                  <span className="text-status-fail diatype-xs-bold">
                    {m["vaultLiquidity.full"]()}
                  </span>
                )}
              </div>
            )}
          </div>
        </div>
        <img
          src="/images/characters/hippo.svg"
          alt="dango-hippo"
          className="max-w-[200px] absolute opacity-10 right-[-2rem] top-[-1rem] select-none drag-none"
        />
      </div>
    </div>
  );
};

const VaultLiquidityContent: React.FC = () => {
  const { state } = useVaultLiquidity();
  const { action, onChangeAction, userHasShares } = state;

  useEffect(() => {
    if (!userHasShares && action === "withdraw") {
      onChangeAction("deposit");
    }
  }, [userHasShares, action, onChangeAction]);

  return (
    <div className="flex flex-col gap-4">
      <Tabs
        layoutId="tabs-vault-deposit-withdraw"
        selectedTab={action}
        keys={userHasShares ? ["deposit", "withdraw"] : ["deposit"]}
        fullWidth
        onTabChange={(tab) => onChangeAction(tab as "deposit" | "withdraw")}
      />
      {action === "deposit" ? <DepositForm /> : <WithdrawForm />}
    </div>
  );
};

const DepositForm: React.FC = () => {
  const { settings, showModal } = useApp();
  const { state, controllers } = useVaultLiquidity();
  const { formatNumberOptions } = settings;
  const { isPaused, isTvlCapReached, userMargin, sharesToReceive, deposit } = state;
  const { register, setValue, errors } = controllers;
  const { account } = useAccount();

  const isLoggedIn = !!account;
  const depositAmount = controllers.inputs.depositAmount?.value || "0";

  useEffect(() => {
    setValue("depositAmount", "");
  }, [account?.address, setValue]);

  return (
    <>
      <div className="flex flex-col gap-2">
        <p className="exposure-sm-italic text-ink-secondary-700">
          {m["vaultLiquidity.toDeposit"]()}
        </p>
        <div className="group flex flex-col rounded-xl bg-surface-secondary-rice shadow-account-card">
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
              inputWrapper: "pl-0 py-3 flex-col h-auto gap-[6px] bg-transparent shadow-none",
              input: "!h3-medium",
            }}
            insideBottomComponent={
              isLoggedIn ? (
                <div className="flex flex-col w-full gap-3 pl-4 pr-4 pb-2">
                  <div className="flex items-center justify-between text-ink-tertiary-500 diatype-sm-regular">
                    <span>
                      {m["vaultLiquidity.availableToDeposit"]()}:{" "}
                      {formatNumber(userMargin, { ...formatNumberOptions, currency: "USD" })}
                    </span>
                  </div>
                  <RangeWithButtons
                    amount={depositAmount}
                    balance={userMargin}
                    setValue={(v: string) => setValue("depositAmount", v)}
                    setActiveInput={() => {}}
                    classNames={{
                      track: "group-hover:bg-outline-primary-gray transition-colors",
                    }}
                  />
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
        <p className="exposure-sm-italic text-ink-secondary-700">{m["vaultLiquidity.toGet"]()}</p>
        <div className="flex items-center justify-between p-4 rounded-xl bg-surface-secondary-rice shadow-account-card">
          <p className="text-ink-tertiary-500 diatype-m-regular">
            {m["vaultLiquidity.vaultShares"]()}
          </p>
          <p className="text-ink-secondary-700 h3-medium">
            {formatNumber(sharesToReceive, formatNumberOptions)}
          </p>
        </div>
      </div>

      {isTvlCapReached ? (
        <WarningContainer
          color="error"
          className="border border-outline-primary-red"
          description={m["vaultLiquidity.tvlCapReached"]()}
        />
      ) : (
        <WarningContainer
          color="error"
          className="border border-outline-primary-red"
          description={
            <ul className="list-disc pl-4 flex flex-col gap-1">
              <li>
                {m["vaultLiquidity.riskWarning1Pre"]()}
                <strong>{m["vaultLiquidity.riskWarning1Bold"]()}</strong>
                {m["vaultLiquidity.riskWarning1Post"]()}
              </li>
              <li>
                {m["vaultLiquidity.riskWarning2Pre"]()}
                <strong>{m["vaultLiquidity.riskWarning2Bold"]()}</strong>
                {m["vaultLiquidity.riskWarning2Post"]()}
              </li>
              <li>
                {m["vaultLiquidity.riskWarning3Pre"]()}
                <strong>{m["vaultLiquidity.riskWarning3Bold"]()}</strong>
              </li>
            </ul>
          }
        />
      )}

      {isLoggedIn ? (
        <Button
          size="md"
          fullWidth
          isDisabled={isPaused || isTvlCapReached || Number(depositAmount) <= 0}
          isLoading={deposit.isPending}
          onClick={() =>
            showModal(Modals.VaultAddLiquidity, {
              amount: depositAmount,
              sharesToReceive,
              confirmAddLiquidity: deposit.mutateAsync,
            })
          }
        >
          {m["common.deposit"]()}
        </Button>
      ) : (
        <Button size="md" fullWidth onClick={() => showModal(Modals.Authenticate)}>
          {m["common.signin"]()}
        </Button>
      )}
    </>
  );
};

const WithdrawForm: React.FC = () => {
  const { settings, showModal } = useApp();
  const { state, controllers } = useVaultLiquidity();
  const { formatNumberOptions } = settings;
  const { isPaused, userVaultShares, usdToReceive, withdraw } = state;
  const { setValue } = controllers;
  const { account } = useAccount();
  const [percentage, setPercentage] = useState(0);

  const isLoggedIn = !!account;
  const withdrawShares =
    percentage === 100
      ? userVaultShares
      : String(Math.floor((Number(userVaultShares) * percentage) / 100));

  useEffect(() => {
    setValue("withdrawShares", withdrawShares);
  }, [withdrawShares, setValue]);

  useEffect(() => {
    if (withdraw.isSuccess) {
      setPercentage(0);
    }
  }, [withdraw.isSuccess]);

  useEffect(() => {
    setPercentage(0);
  }, [account?.address]);

  const handlePercentageChange = (newPercentage: number) => {
    setPercentage(Math.min(100, Math.max(0, newPercentage)));
  };

  return (
    <>
      <div className="flex flex-col gap-2">
        <p className="exposure-sm-italic text-ink-secondary-700">
          {m["vaultLiquidity.withdrawalAmount"]()}
        </p>
        <div className="group flex flex-col gap-4 p-4 rounded-xl bg-surface-secondary-rice shadow-account-card">
          <p className="text-ink-secondary-700 h1-regular text-center">{percentage}%</p>
          <div className="flex flex-col gap-3">
            <RangeWithButtons
              amount={String(percentage)}
              balance="100"
              setValue={(v: string) => handlePercentageChange(Number(v))}
              setActiveInput={() => {}}
              classNames={{
                track: "group-hover:bg-outline-primary-gray transition-colors",
              }}
            />
          </div>
        </div>
      </div>

      <div className="flex flex-col gap-2 text-ink-tertiary-500 diatype-sm-regular">
        <div className="flex justify-between">
          <span>{m["vaultLiquidity.vaultShare"]()}</span>
          <span>{formatNumber(withdrawShares, formatNumberOptions)}</span>
        </div>
        <div className="flex justify-between">
          <span>{m["vaultLiquidity.networkFee"]()}</span>
          <span>{formatNumber("0.00", { ...formatNumberOptions, currency: "USD" })}</span>
        </div>
      </div>

      {isLoggedIn ? (
        <Button
          size="md"
          fullWidth
          isDisabled={isPaused || percentage <= 0}
          isLoading={withdraw.isPending}
          onClick={() =>
            showModal(Modals.VaultWithdrawLiquidity, {
              sharesToBurn: withdrawShares,
              usdToReceive,
              confirmWithdrawal: withdraw.mutateAsync,
            })
          }
        >
          {m["common.withdraw"]()}
        </Button>
      ) : (
        <Button size="md" fullWidth onClick={() => showModal(Modals.Authenticate)}>
          {m["common.signin"]()}
        </Button>
      )}
    </>
  );
};

const UserPosition: React.FC = () => {
  const { state } = useVaultLiquidity();
  const { settings } = useApp();
  const { formatNumberOptions } = settings;
  const { userHasShares, userSharesValue, userUnlocks } = state;

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-4 p-4 rounded-xl bg-surface-secondary-rice shadow-account-card">
        <p className="exposure-sm-italic text-ink-tertiary-500">
          {m["vaultLiquidity.liquidity"]()}
        </p>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-1">
            <img
              src={perpsMarginAsset.logoURI}
              alt={perpsMarginAsset.symbol}
              className="w-5 h-5 rounded-full"
            />
            <span className="text-ink-tertiary-500 diatype-m-regular">
              {perpsMarginAsset.symbol}
            </span>
          </div>
          <span className="text-ink-tertiary-500 diatype-m-regular">
            {formatNumber(userHasShares ? userSharesValue : "0", {
              ...formatNumberOptions,
              currency: "USD",
            })}
          </span>
        </div>
      </div>

      <UserWithdrawals unlocks={userUnlocks} />
    </div>
  );
};

export const VaultLiquidity = Object.assign(VaultLiquidityContainer, {
  Header: VaultLiquidityHeader,
  Content: VaultLiquidityContent,
});
