import { forwardRef } from "react";
import { useMutation } from "@tanstack/react-query";

import {
  Button,
  IconButton,
  IconCheckedCircle,
  IconClose,
  IconUser,
  useApp,
} from "@left-curve/applets-kit";
import { useAccount, useBalances, useConfig } from "@left-curve/store";

import { m } from "@left-curve/foundation/paraglide/messages.js";

export const ActivateAccount = forwardRef<unknown>((_props, _ref) => {
  const { hideModal, navigate, toast } = useApp();
  const { username, account, refreshUserStatus } = useAccount();
  const { chain } = useConfig();
  const { data: balances, refetch: refetchBalances } = useBalances({
    address: account?.address,
  });

  const isMainnet = chain.id === "dango-1";
  const hasBalance = balances && Object.keys(balances).length > 0;

  const { mutate: claimFaucet, isPending } = useMutation({
    mutationFn: async () => {
      const res = await fetch(`${window.dango.urls.faucetUrl}/${account!.address}?skip_check=true`);
      if (!res.ok) throw new Error(await res.text());
    },
    onSuccess: () => {
      refreshUserStatus?.();
      refetchBalances();
    },
    onError: (err) => {
      toast.error({ title: m["common.error"](), description: err.message });
    },
  });

  return (
    <div className="flex flex-col justify-start items-center bg-surface-primary-rice text-ink-primary-900 md:border border-outline-secondary-gray rounded-xl relative px-6 py-8 md:py-6 md:pt-6 gap-6 md:w-[30rem] md:h-fit">
      <IconButton
        className="hidden md:block absolute right-4 top-4"
        variant="link"
        onClick={hideModal}
      >
        <IconClose />
      </IconButton>

      <div className="flex items-center flex-col gap-3">
        <IconCheckedCircle className="w-12 h-12" />
        <h2 className="h2-heavy">{m["signup.deposit.title"]()}</h2>
        <p className="diatype-m-regular text-ink-tertiary-500">
          {m["signup.deposit.description"]()}
        </p>
      </div>

      <div className="flex flex-col gap-3 w-full">
        <div className="flex items-start gap-3 p-4 bg-surface-quaternary-green/40 rounded-xl">
          <div className="h-10 w-10 shrink-0 rounded-full bg-[#E8B86D]/20 flex items-center justify-center">
            <IconUser className="h-5 w-5 text-[#C4893B]" />
          </div>
          <div className="flex flex-col gap-0.5">
            <p className="diatype-m-bold">
              {m["signup.deposit.usernameTitle"]({ username: `#${username}` })}
            </p>
            <p className="diatype-sm-regular text-ink-tertiary-500">
              {m["signup.deposit.usernameDescription"]()}
            </p>
          </div>
        </div>

        {isMainnet ? (
          <div className="flex items-start gap-3 p-4 bg-surface-quaternary-green/40 rounded-xl">
            <div className="h-10 w-10 shrink-0 rounded-full bg-surface-quaternary-green flex items-center justify-center">
              <span className="text-lg">💰</span>
            </div>
            <div className="flex flex-col gap-0.5">
              <p className="diatype-m-bold">{m["signup.deposit.depositCardTitle"]()}</p>
              <p className="diatype-sm-regular text-ink-tertiary-500">
                {m["signup.deposit.depositCardDescription"]()}
              </p>
            </div>
          </div>
        ) : !hasBalance ? (
          <div className="flex items-start gap-3 p-4 bg-surface-quaternary-green/40 rounded-xl">
            <div className="h-10 w-10 shrink-0 rounded-full bg-surface-quaternary-green flex items-center justify-center">
              <span className="text-lg">🚰</span>
            </div>
            <div className="flex flex-col gap-0.5">
              <p className="diatype-m-bold">{m["signup.faucet.cardTitle"]()}</p>
              <p className="diatype-sm-regular text-ink-tertiary-500">
                {m["signup.faucet.cardDescription"]()}
              </p>
            </div>
          </div>
        ) : null}
      </div>

      <div className="flex flex-col items-center gap-3 w-full">
        {isMainnet ? (
          <Button
            fullWidth
            onClick={() => {
              hideModal();
              navigate("/bridge");
            }}
          >
            {m["signup.deposit.cta"]()}
          </Button>
        ) : hasBalance ? (
          <Button
            fullWidth
            onClick={() => {
              hideModal();
              navigate("/settings");
            }}
          >
            {m["signup.faucet.changeUsername"]()}
          </Button>
        ) : (
          <Button fullWidth disabled={isPending} onClick={() => claimFaucet()}>
            {isPending ? m["signup.faucet.claiming"]() : m["signup.faucet.cta"]()}
          </Button>
        )}
        <Button variant="link" onClick={hideModal}>
          <p className="italic">{m["signup.doThisLater"]()}</p>
        </Button>
      </div>
    </div>
  );
});
