import { IconChecked, twMerge } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import { CHARACTERS } from "./constants.js";

export function CharacterSelector({
  selected,
  onSelect,
}: {
  selected: number;
  onSelect: (idx: number) => void;
}) {
  return (
    <div className="flex flex-col gap-2">
      <span className="exposure-sm-italic text-ink-tertiary-500">
        {m["modals.pnlShare.overlay"]()}
      </span>
      <div className="grid grid-cols-5 gap-2">
        {CHARACTERS.map((img, idx) => (
          <button
            key={img}
            type="button"
            onClick={() => onSelect(idx)}
            className={twMerge(
              "relative w-12 h-12 md:w-10 md:h-10 rounded-lg overflow-hidden border-2 cursor-pointer transition-colors",
              idx === selected
                ? "border-primitives-red-light-500"
                : "border-outline-secondary-gray hover:border-ink-tertiary-500",
            )}
          >
            <img
              src={`/images/pnl-modal/${img}.png`}
              alt={img}
              className="w-full h-full object-cover"
            />
            {idx === selected && (
              <div className="absolute top-0.5 right-0.5 w-4 h-4 rounded-full bg-primitives-red-light-500 flex items-center justify-center">
                <IconChecked className="w-2.5 h-2.5 text-white" />
              </div>
            )}
          </button>
        ))}
      </div>
    </div>
  );
}
