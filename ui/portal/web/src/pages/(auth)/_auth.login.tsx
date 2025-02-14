import { Birdo, Button, Input } from "@left-curve/applets-kit";
import { useChainId, useConnectors } from "@left-curve/store-react";
import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useState } from "react";

export const Route = createFileRoute("/(auth)/_auth/login")({
  component: LoginComponent,
});

function LoginComponent() {
  const [step, setStep] = useState<number>(0);
  const navigate = useNavigate();
  const connectors = useConnectors();
  const [username, setUsername] = useState<string>("");
  const chainId = useChainId();

  return (
    <div className="h-screen w-screen flex items-center justify-center">
      {step === 0 && (
        <div className="flex-1 flex items-center justify-center">
          <div className="w-full max-w-[22.5rem] flex items-center justify-center flex-col gap-8 px-4 lg:px-0">
            <div className="flex flex-col gap-7 items-center justify-center">
              <img src="./images/dango.svg" alt="dango-logo" className="h-[24px]" />
              <h1 className="h2-heavy">Log in</h1>
            </div>
            <div className="flex flex-col gap-6 w-full">
              <Input
                label="Username"
                placeholder="Enter your username"
                value={username}
                onChange={({ target }) => setUsername(target.value)}
              />
              <Button fullWidth onClick={() => setStep(1)}>
                Sign in
              </Button>
            </div>
            <div className="flex items-center">
              <p>Don't have an account? </p>
              <Button variant="link" onClick={() => navigate({ to: "/signup" })}>
                Sign up
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
                <h1 className="h2-heavy">Hi, Username</h1>
                <p className="text-gray-500 diatype-m-medium">
                  Choose any of the credentials that have been associated with your username.
                </p>
              </div>
            </div>
            <div className="flex flex-col gap-6 w-full">
              <Button fullWidth> Connect with Passkey</Button>
              <div className="flex items-center justify-center text-gray-500">
                <span className="flex-1 h-[1px] bg-gray-100" />
                <p className="px-2">OR WALLETS</p>
                <span className="flex-1 h-[1px] bg-gray-100" />
              </div>
              <div className="flex flex-col gap-3 w-full">
                {connectors.map((connector) => {
                  if (connector.type === "passkey") return null;
                  return (
                    <Button
                      key={connector.id}
                      variant="secondary"
                      fullWidth
                      onClick={() => connector.connect({ username, chainId })}
                    >
                      Connect with {connector.name}
                    </Button>
                  );
                })}
              </div>
            </div>
            <div className="flex items-center">
              <Button variant="link" onClick={() => setStep(0)}>
                {/* <IconBack/>  */}
                <span>Back</span>
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
