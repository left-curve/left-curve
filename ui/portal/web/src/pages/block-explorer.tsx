import { createLazyRoute } from "@tanstack/react-router";

import { Button, Emoji, Input } from "@left-curve/portal-shared";

export const BlockExplorerRoute = createLazyRoute("/block-explorer")({
  component: () => {
    return (
      <div className="min-h-full w-full flex-1 flex justify-center z-10 relative p-4">
        <div className="flex flex-1 flex-col items-center justify-center gap-4 w-full md:max-w-2xl">
          <div className="flex flex-col items-center justify-center w-full">
            <div className="dango-grid-6x6-M w-[32rem] h-[32rem] flex flex-col relative items-center gap-8">
              <div className="flex flex-col items-center justify-center gap-4 w-full">
                <p className="font-bold text-typography-black-200 font-diatype-rounded tracking-widest uppercase">
                  Block Explorer
                </p>
                <div className="rounded-full bg-surface-rose-200 flex items-center justify-center min-h-[10.5rem] min-w-[10.5rem]">
                  <Emoji detailed name="map" className="h-[7.5rem] w-[7.5rem]" />
                </div>
              </div>
              <div className="flex flex-col gap-4 w-full">
                <Input placeholder="Search block / Tx / Account" color="purple" />
                <Button>Submit</Button>
              </div>
            </div>
          </div>
        </div>
      </div>
    );
  },
});
