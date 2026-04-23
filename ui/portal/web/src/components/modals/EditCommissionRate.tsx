import { forwardRef, useMemo, useState } from "react";

import {
  Button,
  IconButton,
  IconClose,
  Input,
  Skeleton,
  numberMask,
  useApp,
} from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import {
  useAccount,
  useCommissionRateOverride,
  useReferralParams,
  useReferralSettings,
  useSetFeeShareRatio,
} from "@left-curve/store";

const formatPct = (value: number): string => {
  return Number.isInteger(value)
    ? value.toString()
    : value.toFixed(2).replace(/0+$/, "").replace(/\.$/, "");
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

  const { referralParams } = useReferralParams();

  const isLoading = settingsLoading || overrideLoading;

  const commissionRate =
    override ?? settings?.commissionRate ?? referralParams?.referrerCommissionRates.base ?? "0";
  const commissionPct = Number(commissionRate) * 100;
  const currentShareRatio = Number(settings?.shareRatio ?? "0");
  const currentRefereePct = commissionPct * currentShareRatio;

  const [refereeInput, setRefereeInput] = useState<string | null>(null);

  const refereeValue = refereeInput ?? formatPct(currentRefereePct);
  const parsedReferee = Number(refereeValue);
  const youValue = Number.isNaN(parsedReferee)
    ? ""
    : formatPct(Math.max(0, commissionPct - parsedReferee));

  // Convert back to share_ratio for the contract: share_ratio = referee_absolute / commission
  const newShareRatio =
    commissionPct > 0 && !Number.isNaN(parsedReferee)
      ? (parsedReferee / commissionPct).toString()
      : "0";

  const newRatio = commissionPct > 0 ? parsedReferee / commissionPct : 0;
  const canDecrease = !Number.isNaN(parsedReferee) && newRatio < currentShareRatio;
  const exceedsMax = !Number.isNaN(parsedReferee) && newRatio > 0.5;

  const error = useMemo(() => {
    if (refereeValue.trim() === "" || Number.isNaN(parsedReferee)) return null;
    if (parsedReferee < 0 || parsedReferee > commissionPct)
      return m["referral.editFeeShare.errorExceedsCommission"]();
    if (canDecrease) return m["referral.editFeeShare.errorDecrease"]();
    if (exceedsMax) return m["referral.editFeeShare.errorExceedsMax"]();
    return null;
  }, [canDecrease, exceedsMax, refereeValue, parsedReferee, commissionPct]);

  const { mutate: submitSetFeeShareRatio, isPending } = useSetFeeShareRatio({
    onSuccess: () => hideModal(),
  });

  const handleSave = () => {
    if (error || isPending || refereeValue.trim() === "") return;
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

          <Input
            label={m["referral.editFeeShare.youReceive"]()}
            value={youValue}
            readOnly
            endContent={<span className="text-ink-tertiary-500 diatype-m-medium">%</span>}
          />

          <Input
            label={m["referral.editFeeShare.refereeReceives"]()}
            value={refereeValue}
            onChange={(e) => setRefereeInput(numberMask(e.target.value, refereeValue))}
            endContent={<span className="text-ink-tertiary-500 diatype-m-medium">%</span>}
          />

          {error && <p className="text-utility-error-500 diatype-sm-regular">{error}</p>}
        </div>
      )}

      <Button
        fullWidth
        onClick={handleSave}
        disabled={!!error || isPending || isLoading || refereeValue.trim() === ""}
      >
        {isPending ? m["referral.editFeeShare.saving"]() : m["referral.editFeeShare.save"]()}
      </Button>
    </div>
  );
});
