import { useAccount, useBalances, usePrices, useSessionKey } from "@left-curve/store";
import { useNavigate, useRouter } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";
import { Sheet } from "react-modal-sheet";
import { useApp } from "~/hooks/useApp";

import { motion } from "framer-motion";

import { m } from "~/paraglide/messages";
import { Modals } from "./Modal";

import { AccountCard } from "./AccountCard";

import {
  Button,
  IconAddCross,
  IconButton,
  IconChevronDown,
  IconDoubleChevronRight,
  IconLogOut,
  IconMobile,
  twMerge,
  useClickAway,
  useMediaQuery,
} from "@left-curve/applets-kit";
import { AnimatePresence } from "framer-motion";
import { AssetCard } from "./AssetCard";

const ExportComponent = Object.assign(AccountMenu, {
  Desktop,
  Mobile,
  Assets,
  Selector,
});

export { ExportComponent as AccountMenu };

type AccountMenuProps = {
  backAllowed?: boolean;
};

function AccountMenu({ backAllowed }: AccountMenuProps) {
  const { formatNumberOptions, isSidebarVisible } = useApp();
  const { account } = useAccount();
  const { history } = useRouter();
  const [isAccountSelectorActive, setAccountSelectorActive] = useState(false);

  const { data: balances = {} } = useBalances({ address: account?.address });
  const { calculateBalance } = usePrices();

  const totalBalance = calculateBalance(balances, {
    format: true,
    formatOptions: {
      ...formatNumberOptions,
      currency: "USD",
    },
  });

  useEffect(() => {
    if (!isSidebarVisible) setAccountSelectorActive(false);
  }, [isSidebarVisible]);

  if (!account) return null;

  return (
    <>
      <div className="w-full flex items-center flex-col gap-6 relative md:pt-4">
        <div className="flex flex-col w-full items-center gap-5">
          {backAllowed ? (
            <div className="w-full flex gap-2">
              <IconButton variant="link" onClick={() => history.go(-1)}>
                <IconChevronDown className="rotate-90" />
                <span className="h4-bold">{m["common.accounts"]()} </span>
              </IconButton>
            </div>
          ) : null}
          <AnimatePresence mode="wait">
            <motion.div
              className="flex flex-col items-center h-full w-full"
              key={account.address}
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              transition={{ duration: 0.2 }}
              exit={{ opacity: 0 }}
            >
              <AccountCard
                account={account}
                balance={totalBalance}
                isSelectorActive={isAccountSelectorActive}
                onTriggerAction={() => setAccountSelectorActive(!isAccountSelectorActive)}
              />
            </motion.div>
          </AnimatePresence>
        </div>

        <AnimatePresence mode="wait">
          <motion.div
            className="h-full w-full"
            key={isAccountSelectorActive ? "selector" : "assets"}
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.2 }}
            exit={{ opacity: 0 }}
          >
            {isAccountSelectorActive ? <Selector /> : <Assets />}
          </motion.div>
        </AnimatePresence>
      </div>
    </>
  );
}

export function Desktop() {
  const menuRef = useRef<HTMLDivElement>(null);
  const { setSidebarVisibility, isSidebarVisible } = useApp();

  useClickAway(menuRef, () => setSidebarVisibility(false));

  return (
    <div
      ref={menuRef}
      className={twMerge(
        "transition-all lg:absolute fixed top-0 flex h-[100vh] justify-end z-50 duration-300 w-full lg:max-w-[360px] bg-[linear-gradient(90deg,_rgba(0,_0,_0,_0)_3.2%,_rgba(46,_37,_33,_0.1)_19.64%,_rgba(255,_255,_255,_0.1)_93.91%)]",
        isSidebarVisible ? "right-0" : "right-[-50vw]",
      )}
    >
      <div className="lg:pr-2 lg:py-4 w-full relative z-10">
        <div className="w-full bg-white-100 flex flex-col items-center h-full rounded-t-2xl lg:rounded-2xl border border-gray-100">
          <AccountMenu />
        </div>
      </div>
    </div>
  );
}

export function Mobile() {
  const { isSidebarVisible, setSidebarVisibility } = useApp();

  return (
    <Sheet isOpen={isSidebarVisible} onClose={() => setSidebarVisibility(false)}>
      <Sheet.Container className="!bg-white-100 !rounded-t-2xl !shadow-none">
        <Sheet.Header />
        <Sheet.Content>
          <AccountMenu />
        </Sheet.Content>
      </Sheet.Container>
      <Sheet.Backdrop onTap={() => setSidebarVisibility(false)} />
    </Sheet>
  );
}

function Assets() {
  const { setSidebarVisibility, showModal } = useApp();
  const navigate = useNavigate();
  const { connector, account } = useAccount();
  const { deleteSessionkey } = useSessionKey();
  const { isMd } = useMediaQuery();

  const { data: balances = {} } = useBalances({ address: account?.address });

  return (
    <div className="flex flex-col w-full gap-4 items-center">
      <div className="md:self-end flex gap-2 items-center justify-center w-full px-4">
        <Button
          fullWidth
          size="md"
          onClick={() => [
            navigate({ to: "/send-and-receive", search: { action: "receive" } }),
            setSidebarVisibility(false),
          ]}
        >
          {m["common.fund"]()}
        </Button>
        <Button
          fullWidth
          variant="secondary"
          size="md"
          onClick={() => [
            navigate({ to: "/send-and-receive", search: { action: "send" } }),
            setSidebarVisibility(false),
          ]}
        >
          {m["common.send"]()}
        </Button>
        {isMd ? (
          <IconButton variant="secondary" onClick={() => showModal(Modals.QRConnect)}>
            <IconMobile />
          </IconButton>
        ) : null}
        <IconButton
          variant="secondary"
          onClick={() => {
            setSidebarVisibility(false);
            connector?.disconnect();
            deleteSessionkey();
          }}
        >
          <IconLogOut />
        </IconButton>
      </div>
      <div className="flex flex-col w-full overflow-y-auto scrollbar-none pb-4">
        {Object.entries(balances).map(([denom, amount]) => (
          <AssetCard key={denom} coin={{ amount, denom }} />
        ))}
      </div>
    </div>
  );
}

function Selector() {
  const { setSidebarVisibility } = useApp();
  const navigate = useNavigate();
  const { account, accounts, changeAccount } = useAccount();

  if (!account) return null;

  return (
    <div className="flex flex-col w-full gap-4 items-center">
      <div className="flex items-center justify-between gap-4 w-full max-w-[22.5rem] md:max-w-[20.5rem]">
        <p className="diatype-m-bold text-gray-500">{m["accountMenu.accounts.otherAccounts"]()}</p>
        <Button onClick={() => [setSidebarVisibility(false), navigate({ to: "/create-account" })]}>
          <IconAddCross className="w-5 h-5" /> <span>{m["accountMenu.accounts.addAccount"]()}</span>
        </Button>
      </div>
      <div className="flex flex-col items-center w-full overflow-y-auto gap-4 scrollbar-none pb-[7rem] relative">
        {accounts
          ?.filter((acc) => acc.address !== account.address)
          .map((account) => (
            <AccountCard.Preview
              key={account.address}
              account={account}
              onAccountSelect={(acc) => changeAccount?.(acc)}
            />
          ))}
      </div>
    </div>
  );
}
