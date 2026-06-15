import { IconChecked, twMerge } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { Image } from "~/components/foundation/Image";

export const CHARACTERS = [
  "frog1",
  "clouds",
  "frog2",
  "dog1",
  "bird2",
  "rich-cocodrile",
  "hippo",
  "pig",
  "candle",
  "bird-lost",
  "bird-militar",
  "cocodrile_winner",
  "dango-king",
  "dango_winner",
  "pirate",
  "smoke-cocodrile",
] as const;

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
        {m["modals.shareCard.overlay"]()}
      </span>
      <div className="flex flex-wrap gap-2">
        {CHARACTERS.map((img, idx) => (
          <button
            key={img}
            type="button"
            onClick={() => onSelect(idx)}
            className={twMerge(
              "relative w-[62.6px] h-[62.6px] md:w-14 md:h-14 rounded-lg overflow-hidden border cursor-pointer transition-colors",
              idx === selected
                ? "border-primitives-red-light-500"
                : "border-outline-secondary-gray hover:border-ink-tertiary-500",
            )}
          >
            <Image
              src={`/images/pnl-modal-thumb/${img}.png`}
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
