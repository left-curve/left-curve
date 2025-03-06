import { Button, IconAlert } from "@left-curve/applets-kit";
import { useRouter } from "@tanstack/react-router";
import type React from "react";

export const SignupMobile: React.FC = () => {
  const { history } = useRouter();
  return (
    <div className="md:hidden w-screen h-screen bg-gray-900/50 fixed top-0 left-0 z-50 flex items-center justify-center p-4">
      <div className="w-full flex flex-col items-center justify-start bg-white-100 rounded-3xl border border-gray-100 max-w-96">
        <div className="flex flex-col gap-4 p-4 border-b border-b-gray-100">
          <div className="w-12 h-12 bg-error-100 rounded-full flex items-center justify-center">
            <IconAlert className="w-6 h-6 text-error-500" />
          </div>
          <p className="h4-bold">Sign Up Not Available!</p>
          <p className="diatype-m-medium text-gray-500">
            Unfortunately, sign-up is not supported on mobile devices. Please use a desktop to
            complete your sign-up.
          </p>
        </div>
        <div className="p-4 w-full">
          <Button variant="secondary" fullWidth onClick={() => history.go(-1)}>
            Cancel
          </Button>
        </div>
      </div>
    </div>
  );
};
