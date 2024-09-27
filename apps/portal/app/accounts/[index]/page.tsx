import React from "react";
import { AccountRouter } from "~/components/AccountRouter";

export const runtime = "edge";

function AccountPage({ params }: { params: { index: string } }) {
  const { index } = params;
  return (
    <div className="min-h-full w-full flex-1 flex justify-center z-10 relative p-4">
      <div className="flex flex-1 flex-col items-center justify-center gap-4 w-full md:max-w-2xl">
        <AccountRouter index={Number.parseInt(index)} />
      </div>
    </div>
  );
}

export default AccountPage;
