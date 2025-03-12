import { AccountType } from "@left-curve/dango/types";
import { useState } from "react";

import type { AccountTypes } from "@left-curve/dango/types";

import { Button, useWizard } from "@left-curve/applets-kit";
import { SelectorCreateAccount } from "./SelectorCreateAccount";

import { m } from "~/paraglide/messages";

import type React from "react";

export const CreateAccountTypeStep: React.FC = () => {
  const { nextStep, setData } = useWizard();
  const [selectedAccount, setSelectedAccount] = useState<AccountTypes>(AccountType.Spot);

  return (
    <div className="flex w-full flex-col gap-8">
      <div className="flex flex-col gap-6 w-full">
        <SelectorCreateAccount
          accountType={AccountType.Spot}
          onClick={() => setSelectedAccount(AccountType.Spot)}
          isSelected={selectedAccount === AccountType.Spot}
        />
        <SelectorCreateAccount
          accountType={AccountType.Margin}
          onClick={() => setSelectedAccount(AccountType.Margin)}
          isSelected={selectedAccount === AccountType.Margin}
        />
      </div>
      <Button fullWidth onClick={() => [nextStep(), setData({ accountType: selectedAccount })]}>
        {m["common.continue"]()}
      </Button>
    </div>
  );
};
