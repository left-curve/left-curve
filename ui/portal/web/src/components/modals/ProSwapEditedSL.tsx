import { Button, Checkbox, IconButton, IconClose, Input, Range } from "@left-curve/applets-kit";
import { useApp } from "~/hooks/useApp";

import { forwardRef } from "react";

export const ProSwapEditedSL = forwardRef(() => {
  const { hideModal } = useApp();

  return (
    <div className="flex flex-col bg-surface-primary-rice md:border border-secondary-gray pt-0 md:pt-6 rounded-xl relative p-4 md:p-6 gap-5 w-full md:max-w-[30rem]">
      <h2 className="text-primary-900 h4-bold w-full">TP/SL for Position</h2>

      <div className="flex flex-col  gap-1">
        <div className="w-full flex gap-2 items-center justify-between">
          <p className="diatype-sm-regular text-tertiary-500">Coin</p>
          <p className="diatype-sm-medium text-secondary-700">ETH</p>
        </div>
        <div className="w-full flex gap-2 items-center justify-between">
          <p className="diatype-sm-regular text-tertiary-500">Position</p>
          <p className="diatype-sm-medium text-status-success">1.23 ETH</p>
        </div>
        <div className="w-full flex gap-2 items-center justify-between">
          <p className="diatype-sm-regular text-tertiary-500">Entry Price</p>
          <p className="diatype-sm-medium text-secondary-700">82.145</p>
        </div>
        <div className="w-full flex gap-2 items-center justify-between">
          <p className="diatype-sm-regular text-tertiary-500">Mark Price</p>
          <p className="diatype-sm-medium text-secondary-700">82.145</p>
        </div>
        <div className="w-full flex gap-2 items-center justify-between">
          <p className="diatype-sm-regular text-tertiary-500">Stop Loss</p>
          <div className="flex flex-col items-end">
            <p className="diatype-sm-medium text-secondary-700">Price below 7000</p>
            <p className="diatype-sm-medium text-secondary-700">Expected loss: -2.06 USDC</p>
          </div>
        </div>
      </div>
      <div className="flex flex-col gap-4">
        <div className="flex flex-col gap-1">
          <div className="flex gap-2">
            <Input placeholder="0" label="TP Price" />
            <Input
              placeholder="0"
              label="Gain"
              classNames={{ base: "max-w-[6rem]" }}
              /* endContent={
                <Select
                  defaultValue="%"
                  classNames={{ base: "min-w-[4rem]", trigger: "shadow-none" }}
                >
                  <Select.Item value="%">%</Select.Item>
                  <Select.Item value="USDC">USDC</Select.Item>
                </Select>
              } */
            />
          </div>
          <p className="text-tertiary-500 diatype-sm-regular text-right">
            Expected profit: 0.00 USDC
          </p>
        </div>

        <Checkbox checked label="Configure Amount" radius="md" />
        <Range
          minValue={0}
          maxValue={100}
          defaultValue={25}
          inputEndContent="ETH"
          withInput
          classNames={{ input: "max-w-[10rem]" }}
        />
      </div>
      <div className="flex flex-col gap-1">
        <p className="diatype-xs-regular text-tertiary-500">
          By default take-profit and stop-loss orders apply to the entire position. Take-profit and
          stop-loss automatically cancel after closing the position. A market order is triggered
          when the stop loss or take profit price is reached.
        </p>
        <p className="diatype-xs-regular text-tertiary-500">
          If the order size is configured above, the TP/SL order will be for that size no matter how
          the position changes in the future.
        </p>
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
