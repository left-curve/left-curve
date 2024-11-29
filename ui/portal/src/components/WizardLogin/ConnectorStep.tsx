import { Button, Select, SelectItem, useWizard } from "@dango/shared";
import { useConfig, useConnectors } from "@left-curve/react";
import { useMutation } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";

export const ConnectorStep: React.FC = () => {
  const connectors = useConnectors();
  const { chains } = useConfig();
  const navigate = useNavigate();
  const { previousStep, data, setData } = useWizard<{ username: string; retry: boolean }>();
  const { username } = data;

  const { mutateAsync: connect, isPending } = useMutation({
    mutationFn: async (connectorId: string) => {
      const connector = connectors.find((connector) => connector.id === connectorId);
      if (!connector) throw new Error("error: missing connector");
      try {
        await connector.connect({
          username,
          chainId: chains.at(0)!.id,
          challenge: "Please sign this message to confirm your identity.",
        });
        navigate("/");
      } catch (err) {
        console.error(err);
        setData({ retry: true });
        previousStep();
      }
    },
  });

  return (
    <div className="flex flex-col w-full gap-6">
      <Button fullWidth onClick={() => connect("passkey")} isLoading={isPending}>
        Connect with Passkey
      </Button>
      <Select
        label="login-methods"
        placeholder="Alternative sign up methods"
        isDisabled={isPending}
        position="static"
        onSelectionChange={(connectorId) => connect(connectorId.toString())}
      >
        {connectors
          .filter((c) => c.id !== "passkey")
          .map((connector) => {
            return (
              <SelectItem key={connector.id}>
                <div className="flex gap-2">
                  <img
                    src={connector.icon}
                    aria-label="connector-image"
                    className="w-6 h-6 rounded"
                  />
                  <span>{connector.name}</span>
                </div>
              </SelectItem>
            );
          })}
      </Select>
    </div>
  );
};
