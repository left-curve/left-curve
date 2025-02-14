import { Birdo, Button, IconChevronDown, Input, twMerge } from "@left-curve/applets-kit";
import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useState } from "react";

export const Route = createFileRoute("/(auth)/_auth/signup")({
  component: SignupComponent,
});

function SignupComponent() {
  const [step, setStep] = useState<number>(0);
  const [expandWallets, setExpandWallets] = useState<boolean>(false);
  const navigate = useNavigate();

  return (
    <div className="h-screen w-screen flex items-center justify-center">
      {step === 0 && (
        <div className="flex-1 flex items-center justify-center">
          <div className="w-full max-w-[22.5rem] flex items-center justify-center flex-col gap-8 px-4 lg:px-0">
            <div className="flex flex-col gap-7 items-center justify-center">
              <img src="./images/dango.svg" alt="dango-logo" className="h-[24px]" />
              <div className="flex flex-col gap-3 items-center justify-center text-center">
                <h1 className="h2-heavy">Create Your Account</h1>
                <p className="text-gray-500 diatype-m-medium">
                  Choose a log in credential. You can add or remove credentials afterwards.
                </p>
              </div>
            </div>
            <div className="flex flex-col gap-6 w-full">
              <Button fullWidth onClick={() => setStep(1)}>
                Create a passkey
              </Button>
              <div className="flex items-center justify-center text-gray-500">
                <span className="flex-1 h-[1px] bg-gray-100" />
                <div
                  className="flex items-center justify-center gap-1 px-2 cursor-pointer"
                  onClick={() => setExpandWallets(!expandWallets)}
                >
                  <p>OR WALLETS</p>
                  <IconChevronDown
                    className={twMerge(
                      "w-4 h-4 transition-all duration-300",
                      expandWallets ? "rotate-180" : "rotate-0",
                    )}
                  />
                </div>
                <span className="flex-1 h-[1px] bg-gray-100" />
              </div>
              <div
                className={twMerge(
                  "flex items-center flex-col overflow-hidden gap-3 transition-all duration-300",
                  expandWallets ? "h-[16rem]" : "h-0",
                )}
              >
                <Button fullWidth variant="secondary">
                  Phantom
                </Button>
                <Button fullWidth variant="secondary">
                  Backpack
                </Button>
                <Button fullWidth variant="secondary">
                  Metamask
                </Button>
                <Button fullWidth variant="secondary">
                  Keplr
                </Button>
                <Button fullWidth variant="secondary">
                  WalletConnect
                </Button>
              </div>
            </div>
            <div className="flex items-center">
              <p>Already have an account? </p>
              <Button variant="link" onClick={() => navigate({ to: "/login" })}>
                Log in
              </Button>
            </div>
          </div>
        </div>
      )}
      {step === 1 && (
        <div className="flex-1 flex items-center justify-center">
          <div className="w-full max-w-[22.5rem] flex items-center justify-center flex-col gap-8 px-4 lg:px-0">
            <div className="flex flex-col gap-7 items-center justify-center">
              <img src="./images/dango.svg" alt="dango-logo" className="h-[24px]" />
              <div className="flex flex-col gap-3 items-center justify-center text-center">
                <h1 className="h2-heavy">Create Your Account</h1>
                <p className="text-gray-500 diatype-m-medium">
                  Choose a log in credential. You can add or remove credentials afterwards.
                </p>
              </div>
            </div>
            <div className="flex flex-col gap-6 w-full">
              <Button fullWidth onClick={() => setStep(2)}>
                Connect Bitcoin Wallet
              </Button>
            </div>
            <div className="flex items-center flex-col">
              <Button variant="link">Do this later</Button>
              <Button variant="link" onClick={() => setStep(0)}>
                Back
              </Button>
            </div>
          </div>
        </div>
      )}
      {step === 2 && (
        <div className="flex-1 flex items-center justify-center">
          <div className="w-full max-w-[22.5rem] flex items-center justify-center flex-col gap-8 px-4 lg:px-0">
            <div className="flex flex-col gap-7 items-center justify-center">
              <img src="./images/dango.svg" alt="dango-logo" className="h-[24px]" />
              <div className="flex flex-col gap-3 items-center justify-center text-center">
                <h1 className="h2-heavy">Create Your Account</h1>
                <p className="text-gray-500 diatype-m-medium">
                  Choose a log in credential. You can add or remove credentials afterwards.
                </p>
              </div>
            </div>
            <div className="flex flex-col gap-6 w-full">
              <Input label="Username" placeholder="Enter your username" />
              <Button fullWidth onClick={() => setStep(0)}>
                Continue
              </Button>
            </div>
            <div className="flex items-center">
              <Button variant="link" onClick={() => setStep(1)}>
                Back
              </Button>
            </div>
          </div>
        </div>
      )}
      <div className="flex-1 h-full max-w-[1100px] hidden xl:flex bg-[url('./images/frame-rounded.svg')] bg-no-repeat bg-cover bg-center items-center justify-center gap-12 flex-col">
        <Birdo className="max-w-[90%]" />
        <div className="flex flex-col items-center justify-center gap-1">
          <h3 className="exposure-h3-italic">Welcome home</h3>
          <p className="text-gray-500 text-md">The good old days are here to stay.</p>
        </div>
      </div>
    </div>
  );
}
