import { forwardRef, useMemo, useState } from "react";

import { Button, IconButton, IconClose, Input, Skeleton, useApp } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import {
  useAccount,
  useCommissionRateOverride,
  useReferralSettings,
  useSetFeeShareRatio,
} from "@left-curve/store";

const formatPercent = (value: string | undefined): string => {
  if (!value) return "0";
  const num = Number(value);
  if (Number.isNaN(num)) return "0";
  return (num * 100).toFixed(0);
};

export const EditCommissionRate = forwardRef((_props, _ref) => {
  const { hideModal } = useApp();
  const { userIndex } = useAccount();

  const { settings, isLoading: settingsLoading } = useReferralSettings({ userIndex });
  const { override, isLoading: overrideLoading } = useCommissionRateOverride({
    userIndex,
  });

  const isLoading = settingsLoading || overrideLoading;

  const currentSharePercent = formatPercent(settings?.shareRatio);
  const commissionPercent = formatPercent(override ?? settings?.commissionRate);

  const [shareInput, setShareInput] = useState<string | null>(null);

  const shareValue = shareInput ?? currentSharePercent;
  const isShareEmpty = shareValue.trim() === "";
  const parsedSharePercent = Number(shareValue);
  const newShareRatio = isShareEmpty ? null : (parsedSharePercent / 100).toString();

  const currentShareRatio = Number(settings?.shareRatio ?? "0");
  const canDecrease = !isShareEmpty && parsedSharePercent / 100 < currentShareRatio;
  const exceedsMax = !isShareEmpty && parsedSharePercent > 50;

  const error = useMemo(() => {
    if (canDecrease) return m["referral.editCommission.errorDecrease"]();
    if (exceedsMax) return m["referral.editCommission.errorExceedsMax"]();
    return null;
  }, [canDecrease, exceedsMax]);

  const { mutate: submitSetFeeShareRatio, isPending } = useSetFeeShareRatio({
    onSuccess: () => hideModal(),
  });

  const handleSave = () => {
    if (error || isPending || !newShareRatio) return;
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
          {m["referral.editCommission.title"]()}
        </h2>
        <p className="text-ink-tertiary-500 diatype-sm-regular">
          {m["referral.editCommission.description"]()}
        </p>
      </div>

      <div className="w-full h-px bg-outline-secondary-gray" />

      {isLoading ? (
        <div className="flex flex-col gap-4">
          <Skeleton className="w-full h-10" />
          <Skeleton className="w-full h-10" />
        </div>
      ) : (
        <div className="flex flex-col gap-4">
          <p className="text-ink-tertiary-500 diatype-m-regular">
            {m["referral.editCommission.yourRate"]()}{" "}
            <span className="text-utility-success-500 font-bold">{commissionPercent}%</span>{" "}
          </p>

          <Input
            label={m["referral.editCommission.commissionRateLabel"]()}
            value={`${commissionPercent}%`}
            readOnly
          />

          <Input
            label={m["referral.editCommission.refereeReceives"]()}
            value={shareValue}
            onChange={(e) => setShareInput(e.target.value)}
            type="number"
            endContent={<span className="text-ink-tertiary-500 diatype-m-medium">%</span>}
          />

          {error && <p className="text-utility-error-500 diatype-sm-regular">{error}</p>}
        </div>
      )}

      <Button
        fullWidth
        onClick={handleSave}
        disabled={!!error || isPending || isLoading || !newShareRatio}
      >
        {isPending ? m["referral.editCommission.saving"]() : m["referral.editCommission.save"]()}
      </Button>
    </div>
  );
});
