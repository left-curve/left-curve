import { Button, IconButton, IconClose, Input, Range } from "@left-curve/applets-kit";
import { useApp } from "~/hooks/useApp";

import { forwardRef } from "react";

export const ProSwapMarketClose = forwardRef(() => {
  const { hideModal } = useApp();

  return (
    <div className="flex flex-col bg-bg-primary-rice md:border border-secondary-gray pt-0 md:pt-6 rounded-xl relative p-4 md:p-6 gap-6 w-full md:max-w-[25rem]">
      <div className="flex flex-col gap-2">
        <h2 className="text-primary-900 h4-bold w-full">Market Close</h2>
        <p className="text-tertiary-500 diatype-sm-regular">
          This will attempt to immediately close the position.
        </p>
      </div>
      <div>
        <div className="flex flex-col  gap-1">
          <div className="w-full flex gap-2 items-center justify-between">
            <p className="diatype-sm-regular text-tertiary-500">Size</p>
            <p className="diatype-sm-medium text-red-bean-500">1.23 ETH</p>
          </div>
          <div className="w-full flex gap-2 items-center justify-between">
            <p className="diatype-sm-regular text-tertiary-500">Price</p>
            <p className="diatype-sm-medium text-secondary-700">Market</p>
          </div>
        </div>
      </div>
      <div className="flex flex-col gap-4">
        <Input
          placeholder="0"
          label="Size"
          classNames={{
            base: "z-20",
            inputWrapper: "pl-0 py-3 flex-col h-auto gap-[6px]",
            inputParent: "h-[34px] h3-bold",
            input: "!h3-bold",
          }}
          startText="right"
          startContent={
            <div className="inline-flex flex-row items-center gap-3 diatype-m-regular h-[46px] rounded-md min-w-14 p-3 bg-transparent justify-start">
              <div className="flex gap-2 items-center font-semibold">
                <img
                  src="https://raw.githubusercontent.com/cosmos/chain-registry/master/noble/images/USDCoin.svg"
                  alt="usdc"
                  className="w-8 h-8"
                />
                <p>USDC</p>
              </div>
            </div>
          }
        />
        <Range
          minValue={0}
          maxValue={100}
          defaultValue={25}
          withInput
          inputEndContent="%"
          showSteps={[
            { label: "0", value: 0 },
            { label: "25", value: 25 },
            { label: "50", value: 50 },
            { label: "75", value: 75 },
            { label: "100", value: 0 },
          ]}
        />
      </div>
      <IconButton
        className="hidden md:block absolute right-4 top-4"
        variant="link"
        onClick={() => hideModal()}
      >
        <IconClose />
      </IconButton>
      <Button fullWidth onClick={() => hideModal()}>
        Confirm
      </Button>
    </div>
  );
});
