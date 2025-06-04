import { Button, IconButton, IconClose } from "@left-curve/applets-kit";
import { useApp } from "~/hooks/useApp";

import { forwardRef } from "react";

export const ProSwapCloseAll = forwardRef(() => {
  const { hideModal } = useApp();

  return (
    <div className="flex flex-col bg-white-100 md:border border-gray-100 pt-0 md:pt-6 rounded-xl relative p-4 md:p-6 gap-5 w-full md:max-w-[25rem]">
      <h2 className="text-gray-900 h4-bold w-full">Confirm Close All</h2>
      <p className="text-gray-500 diatype-sm-regular">
        This will close all your positions and cancel their associated TP/SL orders.
      </p>
      {/* <RadioGroup name="close-positions-all" defaultValue="market-close">
        <Radio value="market-close" label="Market Close" />
        <Radio value="limit-close" label="Limit Close at Mid Price" />
      </RadioGroup> */}
      <IconButton
        className="hidden md:block absolute right-4 top-4"
        variant="link"
        onClick={() => hideModal()}
      >
        <IconClose />
      </IconButton>
      <Button fullWidth onClick={() => hideModal()}>
        Confirm Market Close
      </Button>
    </div>
  );
});
