import { Button, IconLeft, useWizard } from "@left-curve/applets-kit";
import { useChainId, useConnectors } from "@left-curve/store-react";
import { useMutation } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";

import type React from "react";
import { AuthOptions } from "../AuthOptions";
import { useToast } from "../Toast";

export const LoginCredentialStep: React.FC = () => {
  const connectors = useConnectors();
  const navigate = useNavigate();
  const { toast } = useToast();
  const { data, previousStep } = useWizard<{ username: string }>();
  const chainId = useChainId();

  const { username } = data;

  const { mutateAsync: connect, isPending } = useMutation({
    mutationFn: async (connectorId: string) => {
      const connector = connectors.find((connector) => connector.id === connectorId);
      if (!connector) throw new Error("error: missing connector");
      try {
        await connector.connect({
          username,
          chainId,
          challenge: "Please sign this message to confirm your identity.",
        });
        navigate({ to: "/" });
      } catch (err) {
        console.error(err);
        toast.error({
          title: "Error",
          description: "Failed to connect to the selected credential.",
        });
        // setData({ retry: true, username });
        previousStep();
      }
    },
  });

  return (
    <div className="flex items-center justify-center flex-col gap-8 px-4 lg:px-0">
      <div className="flex flex-col gap-7 items-center justify-center">
        <img src="./images/dango.svg" alt="dango-logo" className="h-[24px]" />
        <div className="flex flex-col gap-3 items-center justify-center text-center">
          <h1 className="h2-heavy">Hi, {username}</h1>
          <p className="text-gray-500 diatype-m-medium">
            Choose any of the credentials that have been associated with your username.
          </p>
        </div>
      </div>
      <AuthOptions action={connect} isPending={isPending} mode="connect" />
      <div className="flex items-center">
        <Button variant="link" onClick={() => previousStep()}>
          <IconLeft className="w-[22px] h-[22px] text-blue-500" />
          <p className="leading-none pt-[2px]">Back</p>
        </Button>
      </div>
    </div>
  );
};
