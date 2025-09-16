import { forwardRef } from "react";
import { useNavigate } from "@tanstack/react-router";

import { AddressVisualizer, Badge, IconNewAccount } from "@left-curve/applets-kit";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type { ActivityRecord } from "@left-curve/store";
import type { ActivityRef } from "./Activity";

type ActivityAccountProps = {
  activity: ActivityRecord<"account">;
};

export const ActivityNewAccount = forwardRef<ActivityRef, ActivityAccountProps>(
  ({ activity }, _) => {
    const navigate = useNavigate();
    const { address, accountType } = activity.data;

    const onNavigate = (url: string) => {
      navigate({ to: url });
    };

    return (
      <div className="flex items-start gap-2 max-w-full overflow-hidden">
        <div className="flex justify-center items-center bg-tertiary-green w-7 h-7 rounded-sm">
          <IconNewAccount className="text-brand-green h-4 w-4" />
        </div>
        <div className="flex flex-col max-w-[calc(100%)] overflow-hidden">
          <div className="flex justify-center items-center gap-2 diatype-m-medium text-secondary-700 capitalize">
            <p>{m["notifications.notification.account.title"]()}</p>
            <Badge className="capitalize" text={accountType} />
          </div>
          <AddressVisualizer address={address} withIcon onClick={onNavigate} />
        </div>
      </div>
    );
  },
);
