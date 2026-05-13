import { Table, useApp, type TableColumn } from "@left-curve/applets-kit";
import { formatNumber } from "@left-curve/dango/utils";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import type { PerpsUnlock } from "@left-curve/dango/types";

type UserWithdrawalsProps = {
  unlocks: PerpsUnlock[];
};

export const UserWithdrawals: React.FC<UserWithdrawalsProps> = ({ unlocks }) => {
  const { settings } = useApp();
  const { formatNumberOptions } = settings;

  const formatDate = (timestamp: string) => {
    const date = new Date(Number(timestamp) * 1000);
    return date.toLocaleDateString("en-US", {
      month: "2-digit",
      day: "2-digit",
      year: "numeric",
    });
  };

  const columns: TableColumn<PerpsUnlock> = [
    {
      id: "amount",
      header: () => (
        <span className="text-ink-tertiary-500 diatype-xs-medium">
          {m["vaultLiquidity.usdAmount"]()}
        </span>
      ),
      cell: ({ row }) => (
        <span className="text-ink-secondary-700 diatype-sm-regular">
          {formatNumber(row.original.amountToRelease, { ...formatNumberOptions, currency: "USD" })}
        </span>
      ),
    },
    {
      id: "endTime",
      header: () => (
        <span className="text-ink-tertiary-500 diatype-xs-medium">
          {m["vaultLiquidity.cooldownEndTime"]()}
        </span>
      ),
      cell: ({ row }) => (
        <span className="text-ink-secondary-700 diatype-sm-regular">
          {formatDate(row.original.endTime)}
        </span>
      ),
    },
  ];

  const hasUnlocks = unlocks.length > 0;

  return (
    <div className="flex flex-col gap-3 p-4 rounded-xl bg-surface-secondary-rice shadow-account-card">
      <p className="exposure-sm-italic text-ink-secondary-700">
        {m["vaultLiquidity.myWithdrawals"]()}
      </p>

      {hasUnlocks ? (
        <>
          <p className="text-ink-tertiary-500 diatype-xs-regular">
            {m["vaultLiquidity.withdrawalsDescription"]()}
          </p>
          <Table
            columns={columns}
            data={unlocks}
            classNames={{
              base: "bg-transparent shadow-none p-0 gap-0",
            }}
          />
        </>
      ) : (
        <div className="flex flex-col items-center gap-2 py-4">
          <img
            src="/images/notifications/no-notifications.svg"
            alt="No withdrawals"
            className="w-[120px] h-auto"
          />
          <p className="exposure-sm-italic text-ink-secondary-700">
            {m["vaultLiquidity.noWithdrawals"]()}
          </p>
          <p className="text-ink-tertiary-500 diatype-sm-medium text-center">
            {m["vaultLiquidity.noWithdrawalsDescription"]()}
          </p>
        </div>
      )}
    </div>
  );
};
