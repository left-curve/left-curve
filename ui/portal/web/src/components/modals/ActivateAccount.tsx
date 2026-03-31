import { forwardRef } from "react";

import {
  Button,
  IconButton,
  IconCheckedCircle,
  IconClose,
  IconUser,
  useApp,
} from "@left-curve/applets-kit";
import { useAccount } from "@left-curve/store";

import { m } from "@left-curve/foundation/paraglide/messages.js";

export const ActivateAccount = forwardRef<unknown>((_props, _ref) => {
  const { hideModal, navigate } = useApp();
  const { username } = useAccount();

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
      </div>

      <div className="flex flex-col items-center gap-3 w-full">
        <Button
          fullWidth
          onClick={() => {
            hideModal();
            navigate("/bridge");
          }}
        >
          {m["signup.deposit.cta"]()}
        </Button>
        <Button variant="link" onClick={hideModal}>
          <p className="italic">{m["signup.doThisLater"]()}</p>
        </Button>
      </div>
    </div>
  );
});
