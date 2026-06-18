import { Badge, IconChevronRight, IconConfetti } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";

export const DepositFreeBadge = () => {
  return (
    <Badge
      color="green"
      size="s"
      className="inline-flex h-6 items-center rounded-full"
      text={
        <span aria-hidden="true" className="inline-flex items-center gap-1">
          {m["bridge.deposit.free"]()}
          <IconConfetti className="h-3 w-3" />
        </span>
      }
    />
  );
};

export const DepositFeeBadge = () => {
  return (
    <Badge
      color="red"
      size="s"
      text={m["bridge.deposit.moreOptions.fee"]()}
      className="shrink-0 rounded-full border"
    />
  );
};

type MoreDepositOptionsCardProps = {
  onClick: () => void;
};

export const MoreDepositOptionsCard = ({ onClick }: MoreDepositOptionsCardProps) => {
  return (
    <button
      type="button"
      onClick={onClick}
      className="flex w-full flex-col gap-2 rounded-xl bg-surface-secondary-rice px-4 py-3 text-left shadow-account-card transition-colors hover:bg-surface-tertiary-rice focus:outline-none focus-visible:ring-2 focus-visible:ring-primitives-red-light-300"
    >
      <span className="flex w-full items-center justify-between gap-3">
        <span className="diatype-m-bold text-ink-secondary-700">
          {m["bridge.deposit.moreOptions.title"]()}
        </span>
        <DepositFeeBadge />
      </span>
      <span className="flex w-full items-center justify-between gap-3">
        <span className="min-w-0 diatype-sm-regular text-ink-tertiary-500">
          {m["bridge.deposit.moreOptions.description"]()}
        </span>
        <span className="inline-flex shrink-0 items-center gap-1 exposure-sm-italic text-ink-secondary-blue">
          {m["bridge.deposit.moreOptions.cta"]()}
          <IconChevronRight className="h-4 w-4" />
        </span>
      </span>
    </button>
  );
};
