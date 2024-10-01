import React from "react";
import { AccountCreation } from "~/components/AccountCreation/AccountCreation";

function HomePage() {
  return (
    <div className="min-h-full w-full flex-1 flex justify-center z-10 relative p-4">
      <div className="flex flex-1 w-full items-center justify-center">
        <AccountCreation />
      </div>
    </div>
  );
}

export default HomePage;
