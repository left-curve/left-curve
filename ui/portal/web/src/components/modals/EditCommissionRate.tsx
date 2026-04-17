import { forwardRef, useMemo, useState } from "react";

import { Button, IconButton, IconClose, Input, Skeleton, numberMask, useApp } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import {
  useAccount,
  useCommissionRateOverride,
  useReferralSettings,
  useSetFeeShareRatio,
} from "@left-curve/store";

const formatPct = (value: number): string => {
  return Number.isInteger(value) ? value.toString() : value.toFixed(2).replace(/0+$/, "").replace(/\.$/, "");
};

const formatPercent = (value: string | undefined): string => {
  if (!value) return "0";
  const num = Number(value);
  if (Number.isNaN(num)) return "0";
  return formatPct(num * 100);
};

export const EditCommissionRate = forwardRef((_props, _ref) => {
  const { hideModal } = useApp();
  const { userIndex } = useAccount();

  const { settings, isLoading: settingsLoading } = useReferralSettings({ userIndex });
  const { override, isLoading: overrideLoading } = useCommissionRateOverride({
    userIndex,
  });

  const isLoading = settingsLoading || overrideLoading;

  const commissionRate = override ?? settings?.commissionRate ?? "0";
  const commissionPct = Number(commissionRate) * 100;
  const currentShareRatio = Number(settings?.shareRatio ?? "0");
  const currentSharePercent = formatPercent(settings?.shareRatio);
  const currentRefereePct = commissionPct * currentShareRatio;
  const currentYouPct = commissionPct * (1 - currentShareRatio);

  const [shareInput, setShareInput] = useState<string | null>(null);

  const shareValue = shareInput ?? currentSharePercent;
  const parsedShare = Number(shareValue);
  const newShareRatio = (parsedShare / 100).toString();

  const youSharePct = Number.isNaN(parsedShare) ? 0 : 100 - parsedShare;
  const refereeSplitPct = Number.isNaN(parsedShare) ? 0 : commissionPct * (parsedShare / 100);
  const youSplitPct = Number.isNaN(parsedShare) ? 0 : commissionPct - refereeSplitPct;

  const canDecrease = !Number.isNaN(parsedShare) && parsedShare / 100 < currentShareRatio;
  const exceedsMax = !Number.isNaN(parsedShare) && parsedShare > 50;

  const error = useMemo(() => {
    if (shareValue.trim() === "" || Number.isNaN(parsedShare)) return null;
    if (canDecrease) return m["referral.editFeeShare.errorDecrease"]();
    if (exceedsMax) return m["referral.editFeeShare.errorExceedsMax"]();
    return null;
  }, [canDecrease, exceedsMax, shareValue, parsedShare]);

  const { mutate: submitSetFeeShareRatio, isPending } = useSetFeeShareRatio({
    onSuccess: () => hideModal(),
  });

  const handleSave = () => {
    if (error || isPending || shareValue.trim() === "") return;
    submitSetFeeShareRatio({ shareRatio: newShareRatio });
  };

  return (
    <div className="flex flex-col bg-surface-primary-rice md:border border-outline-secondary-gray pt-0 md:pt-6 rounded-xl relative p-4 md:p-6 gap-6 w-full md:max-w-[25rem]">
      <IconButton
        className="hidden md:block absolute right-4 top-4"
        variant="link"
        onClick={() => hideModal()}
      >
        <IconClose />
      </IconButton>

      <div className="flex flex-col gap-2">
        <h2 className="text-ink-primary-900 h4-bold w-full">
          {m["referral.editFeeShare.title"]()}
        </h2>
        <p className="text-ink-tertiary-500 diatype-sm-regular">
          {m["referral.editFeeShare.description"]()}
        </p>
      </div>

      <div className="-mx-4 md:-mx-6 h-px bg-outline-secondary-gray" />

      {isLoading ? (
        <div className="flex flex-col gap-4">
          <Skeleton className="w-full h-10" />
          <Skeleton className="w-full h-10" />
        </div>
      ) : (
        <div className="flex flex-col gap-4">
          <p className="diatype-m-regular text-ink-tertiary-500">
            {m["referral.editFeeShare.yourCommissionRate"]()}{" "}
            <span className="text-status-success diatype-m-bold">
              {formatPercent(commissionRate)}%
            </span>
          </p>

          <div className="flex flex-col gap-1">
            <Input
              label={m["referral.editFeeShare.youReceive"]()}
              value={formatPct(youSharePct)}
              readOnly
              endContent={<span className="text-ink-tertiary-500 diatype-m-medium">%</span>}
            />
            <p className="diatype-sm-regular">
              <span className="text-ink-tertiary-500">{m["referral.editFeeShare.current"]()}:</span>{" "}
              <span className="text-utility-success-500">{formatPct(currentYouPct)}%</span>
            </p>
          </div>

          <div className="flex flex-col gap-1">
            <Input
              label={m["referral.editFeeShare.refereeReceives"]()}
              value={shareValue}
              onChange={(e) => setShareInput(numberMask(e.target.value))}
              endContent={<span className="text-ink-tertiary-500 diatype-m-medium">%</span>}
            />
            <p className="diatype-sm-regular">
              <span className="text-ink-tertiary-500">{m["referral.editFeeShare.current"]()}:</span>{" "}
              <span className="text-utility-success-500">{formatPct(currentRefereePct)}%</span>
            </p>
          </div>

          {error && <p className="text-utility-error-500 diatype-sm-regular">{error}</p>}
        </div>
      )}

      <Button
        fullWidth
        onClick={handleSave}
        disabled={!!error || isPending || isLoading || shareValue.trim() === ""}
      >
        {isPending ? m["referral.editFeeShare.saving"]() : m["referral.editFeeShare.save"]()}
      </Button>
    </div>
  );
});
