import { forwardRef, useImperativeHandle } from "react";

import { Button, IconButton, IconClose, twMerge, useApp } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { FALLBACK_CAMPAIGN_MAP } from "@left-curve/store";
import type { OatCheckEntry } from "@left-curve/store";

import type { ModalRef } from "./RootModal";

type OATType = "supporter" | "wizard" | "trader" | "hurrah";

const OATImages: Record<OATType, string> = {
  hurrah: "/images/points/oats/hurrah.png",
  trader: "/images/points/oats/trader.png",
  wizard: "/images/points/oats/wizard.png",
  supporter: "/images/points/oats/supporter.png",
};

const OATTitles: Record<OATType, () => string> = {
  hurrah: () => m["points.boosters.oats.hurrah"](),
  trader: () => m["points.boosters.oats.trader"](),
  wizard: () => m["points.boosters.oats.wizard"](),
  supporter: () => m["points.boosters.oats.supporter"](),
};

type OatCheckResultProps = {
  entries: OatCheckEntry[];
  currentUserIndex: number;
  onConfirm: () => void;
  onReject: () => void;
};

export const OatCheckResult = forwardRef<ModalRef, OatCheckResultProps>(
  ({ entries, currentUserIndex, onConfirm, onReject }, ref) => {
    const { hideModal } = useApp();

    useImperativeHandle(ref, () => ({
      triggerOnClose: () => onReject(),
    }));

    const handleConfirm = () => {
      onConfirm();
      hideModal();
    };

    const handleCancel = () => {
      onReject();
      hideModal();
    };

    if (entries.length === 0) {
      return (
        <div className="flex flex-col bg-surface-primary-rice md:border border-outline-secondary-gray rounded-xl relative p-4 md:p-6 gap-5 w-full md:max-w-[25rem]">
          <p className="text-ink-primary-900 diatype-lg-medium w-full text-center">
            {m["modals.oatCheckResult.title"]()}
          </p>
          <p className="text-ink-tertiary-500 diatype-m-medium text-center">
            {m["modals.oatCheckResult.noOats"]()}
          </p>
          <Button variant="secondary" size="lg" onClick={handleCancel} className="w-full">
            {m["modals.oatCheckResult.cancel"]()}
          </Button>
          <IconButton
            className="hidden md:block absolute right-4 top-4"
            variant="link"
            onClick={handleCancel}
          >
            <IconClose />
          </IconButton>
        </div>
      );
    }

    return (
      <div className="flex flex-col bg-surface-primary-rice md:border border-outline-secondary-gray rounded-xl relative p-4 md:p-6 gap-5 w-full md:max-w-[25rem]">
        <p className="text-ink-primary-900 diatype-lg-medium w-full text-center">
          {m["modals.oatCheckResult.title"]()}
        </p>

        <div className="flex flex-col gap-3">
          {entries.map((entry) => {
            const oatType = FALLBACK_CAMPAIGN_MAP[entry.collection_id];
            const title = oatType ? OATTitles[oatType]() : `OAT #${entry.collection_id}`;
            const imageSrc = oatType ? OATImages[oatType] : undefined;

            const isLinkedToCurrentUser = entry.maybe_username?.index === currentUserIndex;
            const isLinkedToOther =
              entry.maybe_username != null && entry.maybe_username.index !== currentUserIndex;
            const isAvailable = entry.maybe_username == null;

            return (
              <div
                key={entry.collection_id}
                className="flex items-center gap-3 p-3 rounded-lg bg-surface-secondary-rice border border-outline-primary-gray"
              >
                {imageSrc && (
                  <img
                    src={imageSrc}
                    alt={title}
                    className={twMerge(
                      "w-10 h-10 rounded-md object-cover",
                      isLinkedToOther && "opacity-50",
                    )}
                  />
                )}
                <div className="flex flex-col flex-1 min-w-0">
                  <span className="diatype-s-medium text-ink-primary-900 truncate">{title}</span>
                  {isAvailable && (
                    <span className="diatype-xs-regular text-primitives-green-light-400">
                      {m["modals.oatCheckResult.available"]()}
                    </span>
                  )}
                  {isLinkedToCurrentUser && (
                    <span className="diatype-xs-regular text-ink-tertiary-500">
                      {m["modals.oatCheckResult.linkedToYou"]()}
                    </span>
                  )}
                  {isLinkedToOther && (
                    <span className="diatype-xs-regular text-fg-primary-red">
                      {m["modals.oatCheckResult.linkedToUser"]({
                        index: String(entry.maybe_username!.index),
                      })}
                    </span>
                  )}
                </div>
              </div>
            );
          })}
        </div>

        <div className="flex gap-3">
          <Button variant="secondary" size="lg" onClick={handleCancel} className="flex-1">
            {m["modals.oatCheckResult.cancel"]()}
          </Button>
          <Button variant="primary" size="lg" onClick={handleConfirm} className="flex-1">
            {m["modals.oatCheckResult.confirm"]()}
          </Button>
        </div>

        <IconButton
          className="hidden md:block absolute right-4 top-4"
          variant="link"
          onClick={handleCancel}
        >
          <IconClose />
        </IconButton>
      </div>
    );
  },
);
