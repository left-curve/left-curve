import { useInputs, useMediaQuery, useWizard } from "@left-curve/applets-kit";
import { useAccount, useBalances, useConfig, useSigningClient } from "@left-curve/store";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate, useRouter } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { useApp } from "~/hooks/useApp";

import { formatNumber, formatUnits, parseUnits, wait } from "@left-curve/dango/utils";
import { m } from "~/paraglide/messages";
import { toast } from "../foundation/Toast";

import {
  Button,
  IconButton,
  IconCheckedCircle,
  IconChevronDown,
  Input,
  ResizerContainer,
  Stepper,
} from "@left-curve/applets-kit";
import { ensureErrorMessage, twMerge } from "@left-curve/applets-kit";
import { AccountType } from "@left-curve/dango/types";
import { Link } from "@tanstack/react-router";
import { Modals } from "../modals/RootModal";

import type { AccountTypes } from "@left-curve/dango/types";
import type React from "react";

export const Container: React.FC<React.PropsWithChildren> = ({ children }) => {
  const { activeStep } = useWizard();
  const { isMd } = useMediaQuery();
  const { history } = useRouter();

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

const TypeSelector: React.FC = () => {
  const { isConnected } = useAccount();
  const { nextStep, setData } = useWizard();
  const [selectedAccount, setSelectedAccount] = useState<AccountTypes>(AccountType.Spot);

  return (
    <div className="flex w-full flex-col gap-8">
      <div className="flex flex-col gap-6 w-full">
        <AccountTypeSelector
          accountType={AccountType.Spot}
          onClick={() => setSelectedAccount(AccountType.Spot)}
          isSelected={selectedAccount === AccountType.Spot}
        />
        <AccountTypeSelector
          accountType={AccountType.Margin}
          onClick={() => setSelectedAccount(AccountType.Margin)}
          isSelected={selectedAccount === AccountType.Margin}
        />
      </div>
      {isConnected ? (
        <Button fullWidth onClick={() => [nextStep(), setData({ accountType: selectedAccount })]}>
          {m["common.continue"]()}
        </Button>
      ) : (
        <Button as={Link} fullWidth to="/signin">
          {m["common.signin"]()}
        </Button>
      )}
    </div>
  );
};

type AccountTypeSelectorProps = {
  accountType: AccountTypes;
  onClick?: () => void;
  isSelected?: boolean;
};

const AccountTypeSelector: React.FC<AccountTypeSelectorProps> = ({
  accountType,
  isSelected,
  onClick,
}) => {
  return (
    <div
      className={twMerge(
        "min-h-[9.125rem] w-full max-w-[22.5rem] border border-transparent text-start rounded-md overflow-hidden relative p-4 flex flex-col gap-4 transition-all shadow-account-card items-start justify-start",
        { "cursor-pointer": onClick },
        { " border border-red-bean-400": isSelected },
        {
          "bg-[linear-gradient(98.89deg,_rgba(255,_251,_245,_0.5)_5.88%,_rgba(249,_226,_226,_0.5)_46.73%,_rgba(255,_251,_244,_0.5)_94.73%)]":
            accountType === "spot",
        },
        {
          "bg-[linear-gradient(0deg,_#FFFCF6,_#FFFCF6),linear-gradient(98.89deg,_rgba(248,_249,_239,_0.5)_5.88%,_rgba(239,_240,_195,_0.5)_46.73%,_rgba(248,_249,_239,_0.5)_94.73%)]":
            accountType === "margin",
        },
      )}
      onClick={onClick}
    >
      <p className="capitalize exposure-m-italic">
        {m["accountCreation.accountType.title"]({ accountType })}
      </p>
      <p className="diatype-sm-medium text-gray-500 relative max-w-[15.5rem] z-10">
        {m["accountCreation.accountType.description"]({ accountType })}
      </p>
      <img
        src={`/images/account-creation/${accountType}.svg`}
        alt={`create-account-${accountType}`}
        className={twMerge("absolute right-0 bottom-0", { "right-2": accountType === "margin" })}
      />
      <IconCheckedCircle
        className={twMerge("w-5 h-5 absolute right-3 top-3 opacity-0 transition-all text-red-400", {
          "opacity-1": isSelected,
        })}
      />
    </div>
  );
};

export const Deposit: React.FC = () => {
  const { previousStep, data } = useWizard<{ accountType: AccountTypes }>();
  const { register, inputs } = useInputs();

  const { value: fundsAmount, error } = inputs.amount || {};

  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const { showModal, subscriptions } = useApp();
  const { coins } = useConfig();
  const { username, account, refreshAccounts, changeAccount } = useAccount();
  const { settings } = useApp();
  const { formatNumberOptions } = settings;
  const { data: signingClient } = useSigningClient();

  const { data: balances = {}, refetch: refreshBalances } = useBalances({
    address: account?.address,
  });

  const { accountType } = data;
  const coinInfo = coins["bridge/usdc"];
  const humanBalance = formatUnits(balances["bridge/usdc"] || 0, coinInfo.decimals);

  const { mutateAsync: send, isPending } = useMutation({
    mutationFn: async () => {
      if (!signingClient) throw new Error("error: no signing client");
      subscriptions.emit("submitTx", { isSubmitting: true });
      try {
        const parsedAmount = parseUnits(fundsAmount || "0", coinInfo.decimals).toString();

        await signingClient.registerAccount({
          sender: account!.address,
          config: { [accountType as "spot"]: { owner: account!.username } },
          funds: {
            "bridge/usdc": parsedAmount,
          },
        });

        await refreshAccounts?.();

        subscriptions.emit("submitTx", {
          isSubmitting: false,
          txResult: { hasSucceeded: true, message: m["accountCreation.accountCreated"]() },
        });

        await refreshBalances();
        queryClient.invalidateQueries({ queryKey: ["quests", account] });
        navigate({ to: "/" });
      } catch (e) {
        console.error(e);
        const error = ensureErrorMessage(e);
        subscriptions.emit("submitTx", {
          isSubmitting: false,
          txResult: { hasSucceeded: false, message: m["signup.errors.couldntCompleteRequest"]() },
        });
        toast.error(
          {
            title: m["signup.errors.couldntCompleteRequest"]() as string,
            description: error,
          },
          {
            duration: Number.POSITIVE_INFINITY,
          },
        );
      }
    },
  });

  useEffect(() => {
    if (!username) return;
    return subscriptions.subscribe("account", {
      params: { username },
      listener: async ({ accounts }) => {
        const account = accounts.at(0)!;
        const parsedAmount = parseUnits(fundsAmount || "0", coinInfo.decimals).toString();

        await wait(300);

        showModal(Modals.ConfirmAccount, {
          amount: parsedAmount,
          accountType: account.accountType,
          accountName: `${username} #${account.accountIndex}`,
          denom: "bridge/usdc",
        });

        changeAccount?.(account.address);
      },
    });
  }, [subscriptions, username, fundsAmount, coinInfo]);

  return (
    <form
      className="flex flex-col gap-6 w-full"
      onSubmit={(e) => {
        e.preventDefault();
        send();
      }}
    >
      <Input
        isDisabled={isPending}
        placeholder="0"
        {...register("amount", {
          strategy: "onChange",
          validate: (v) => {
            if (Number(v) > Number(humanBalance))
              return m["errors.validations.insufficientFunds"]();
            return true;
          },
          mask: (v, prev) => {
            const regex = /^\d+(\.\d{0,18})?$/;
            if (v === "" || regex.test(v)) return v;
            return prev;
          },
        })}
        endContent={
          <div className="flex flex-row items-center gap-1 justify-center">
            <img src={coinInfo.logoURI} className="w-5 h-5" alt={coinInfo.name} />
            <span className="diatype-m-regular text-gray-500 pt-1">{coinInfo.symbol}</span>
          </div>
        }
        bottomComponent={
          <div className="w-full flex justify-between">
            <p>{m["common.available"]()}</p>
            <p className="flex gap-1">
              <span>{coinInfo.symbol}</span>
              <span>{formatNumber(humanBalance, formatNumberOptions)}</span>
            </p>
          </div>
        }
      />
      <div className="flex gap-4">
        <Button type="button" fullWidth onClick={() => previousStep()} isDisabled={isPending}>
          {m["common.back"]()}
        </Button>
        <Button type="submit" fullWidth isLoading={isPending} isDisabled={!!error}>
          {m["common.continue"]()}
        </Button>
      </div>
    </form>
  );
};

export const AccountCreation = Object.assign(Container, {
  TypeSelector,
  Deposit,
});
