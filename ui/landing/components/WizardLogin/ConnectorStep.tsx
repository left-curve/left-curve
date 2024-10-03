"use client";

import { DangoButton, Select, SelectItem, useWizard } from "@dango/shared";
import { useConfig, useConnectors } from "@leftcurve/react";
import { useState } from "react";

export const ConnectorStep: React.FC = () => {
  const [connectorId, setConnectorId] = useState<string>("Passkey");
  const connectors = useConnectors();
  const { chains } = useConfig();
  const { previousStep, data, setData } = useWizard<{ username: string; retry: boolean }>();
  const { username } = data;

  const onSubmit = async () => {
    const connector = connectors.find((connector) => connector.id === connectorId.toLowerCase());
    if (!connector) throw new Error("error: missing connector");
    try {
      await connector.connect({
        username,
        chainId: chains.at(0)!.id,
        challenge: "Please sign this message to confirm your identity.",
      });
      window.location.replace(`https://portal.${window.location.hostname}`);
    } catch (err) {
      console.error(err);
      setData({ retry: true });
      previousStep();
    }
  };

  return (
    <div className="flex flex-col w-full gap-3 md:gap-6">
      <DangoButton fullWidth onClick={onSubmit}>
        Connect with {connectorId}
      </DangoButton>
      <Select
        label="login-methods"
        placeholder="Alternative sign up methods"
        defaultSelectedKey={connectorId}
        onSelectionChange={(key) => setConnectorId(key.toString())}
      >
        <SelectItem key="Passkey">Passkey</SelectItem>
        <SelectItem key="Metamask">Metamask</SelectItem>
      </Select>
      <DangoButton onClick={previousStep} variant="ghost" color="sand" className="text-lg">
        Back
      </DangoButton>
    </div>
  );
};
