import { Button, IconButton, IconClose, twMerge, useApp } from "@left-curve/applets-kit";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { perpsTradeSettingsStore, type MarginMode } from "@left-curve/store";
import { forwardRef, useState } from "react";

type PerpsMarginModeProps = {
  perpsPairId: string;
  pairSymbol: string;
};

export const PerpsMarginMode = forwardRef<void, PerpsMarginModeProps>(
  ({ perpsPairId, pairSymbol }) => {
    const { hideModal } = useApp();
    const currentMode =
      perpsTradeSettingsStore((s) => s.marginModeByPair[perpsPairId]) ?? "cross";
    const setMarginMode = perpsTradeSettingsStore((s) => s.setMarginMode);

    const [selected, setSelected] = useState<MarginMode>(currentMode);

    const onConfirm = () => {
      setMarginMode(perpsPairId, selected);
      hideModal();
    };

    return (
      <div className="flex flex-col bg-surface-primary-rice md:border border-outline-secondary-gray pt-0 md:pt-6 rounded-xl relative p-4 md:p-6 gap-5 w-full md:max-w-[28rem]">
        <h2 className="text-ink-primary-900 diatype-lg-bold w-full">
          {m["modals.marginMode.title"]({ symbol: pairSymbol })}
        </h2>

        <div className="flex flex-col gap-3">
          <MarginOption
            label={m["modals.marginMode.cross"]()}
            description={m["modals.marginMode.crossDescription"]()}
            selected={selected === "cross"}
            onClick={() => setSelected("cross")}
          />
          <MarginOption
            label={m["modals.marginMode.isolated"]()}
            description={m["modals.marginMode.isolatedDescription"]()}
            selected={false}
            disabled
          />
        </div>

        <IconButton
          className="hidden md:block absolute right-4 top-4"
          variant="link"
          onClick={() => hideModal()}
        >
          <IconClose />
        </IconButton>

        <Button fullWidth onClick={onConfirm}>
          {m["modals.marginMode.confirm"]()}
        </Button>
      </div>
    );
  },
);

type MarginOptionProps = {
  label: string;
  description: string;
  selected: boolean;
  disabled?: boolean;
  onClick?: () => void;
};

const MarginOption: React.FC<MarginOptionProps> = ({
  label,
  description,
  selected,
  disabled,
  onClick,
}) => {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className={twMerge(
        "flex flex-col gap-2 p-4 rounded-xl border text-left transition-all",
        disabled
          ? "border-outline-secondary-gray bg-surface-disabled-gray cursor-not-allowed"
          : selected
            ? "border-outline-tertiary-rice bg-surface-secondary-rice"
            : "border-outline-tertiary-rice bg-surface-secondary-rice cursor-pointer hover:border-outline-primary-red",
      )}
    >
      <div className="flex items-center gap-2">
        <div
          className={twMerge(
            "w-5 h-5 rounded-full border-[1.5px] flex items-center justify-center shrink-0",
            selected
              ? "border-primitives-red-light-400"
              : "border-outline-secondary-gray",
          )}
        >
          {selected ? (
            <div className="w-2.5 h-2.5 rounded-full bg-primitives-red-light-400" />
          ) : null}
        </div>
        <p className="diatype-m-medium text-ink-primary-900">{label}</p>
      </div>
      <p className="diatype-sm-regular text-ink-tertiary-500">{description}</p>
    </button>
  );
};
