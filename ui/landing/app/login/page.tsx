import React from "react";
import { WizardLogin } from "~/components/WizardLogin";

function LoginPage() {
  return (
    <div className="min-h-full w-full flex-1 flex justify-center z-10 relative p-4">
      <div className="flex flex-1 w-full items-center justify-center">
        <WizardLogin />
      </div>
    </div>
  );
}

export default LoginPage;
