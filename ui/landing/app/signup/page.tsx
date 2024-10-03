import React from "react";
import { WizardSignup } from "~/components/WizardSignup";

function SignupPage() {
  return (
    <div className="min-h-full w-full flex-1 flex justify-center z-10 relative p-4">
      <div className="flex flex-1 w-full items-center justify-center">
        <WizardSignup />
      </div>
    </div>
  );
}

export default SignupPage;
