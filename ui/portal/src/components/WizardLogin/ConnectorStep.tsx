import { Button, ConnectorButtonOptions, useWizard } from "@dango/shared";
import { useConfig, useConnectors } from "@left-curve/react";
import { useState } from "react";
import { useNavigate } from "react-router-dom";

export const ConnectorStep: React.FC = () => {
  const [connectorLoading, setConnectorLoading] = useState<string>();

  const connectors = useConnectors();
  const { chains } = useConfig();
  const navigate = useNavigate();
  const { previousStep, data, setData } = useWizard<{ username: string; retry: boolean }>();
  const { username } = data;

  const connect = async (connectorId: string) => {
    setConnectorLoading(connectorId);
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
      setConnectorLoading(undefined);
      setData({ retry: true });
      previousStep();
    }
  };

  return (
    <div className="flex flex-col w-full gap-3 md:gap-6">
      <Button
        fullWidth
        onClick={() => connect("passkey")}
        isDisabled={!!connectorLoading}
        isLoading={connectorLoading === "passkey"}
      >
        Connect with Passkey
      </Button>
      <div className="flex flex-col gap-2 w-full">
        <ConnectorButtonOptions
          mode="login"
          connectors={connectors}
          selectedConnector={connectorLoading}
          onClick={connect}
        />
      </div>
    </div>
  );
};
