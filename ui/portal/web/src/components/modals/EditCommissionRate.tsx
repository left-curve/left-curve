import { forwardRef, useState } from "react";

import { Button, IconButton, IconClose, Input, useApp } from "@left-curve/applets-kit";

export const EditCommissionRate = forwardRef((_props, _ref) => {
  const { hideModal } = useApp();
  const [youReceive, setYouReceive] = useState("10");
  const [refereeReceives, setRefereeReceives] = useState("5");

  const totalRate = Number(youReceive || 0) + Number(refereeReceives || 0);

  const handleSave = () => {
    // TODO: Implement save logic
    hideModal();
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
        <h2 className="text-ink-primary-900 h4-bold w-full">Edit Commission Rate</h2>
        <p className="text-ink-tertiary-500 diatype-sm-regular">
          You can change your commission rate only once. You cannot share more than 50% of your
          commission with your referees.
        </p>
      </div>

      <div className="w-full h-px bg-outline-secondary-gray" />

      <div className="flex flex-col gap-4">
        <p className="text-ink-tertiary-500 diatype-m-regular">
          Your commission rate:{" "}
          <span className="text-utility-success-500 font-bold">{totalRate}%</span>
        </p>

        <Input
          label="You receive"
          value={youReceive}
          onChange={(e) => setYouReceive(e.target.value)}
          type="number"
          endContent={<span className="text-ink-tertiary-500 diatype-m-medium">%</span>}
        />

        <Input
          label="Referee receives"
          value={refereeReceives}
          onChange={(e) => setRefereeReceives(e.target.value)}
          type="number"
          endContent={<span className="text-ink-tertiary-500 diatype-m-medium">%</span>}
        />
      </div>

      <Button fullWidth onClick={handleSave}>
        Save
      </Button>
    </div>
  );
});
