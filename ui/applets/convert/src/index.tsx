import { useState } from "react";
import { SimpleSwap } from "./components/Convert";
import { DangoRemoteProvider } from "@left-curve/store";
import { useRemoteApp } from "@left-curve/applets-kit";

export const ConvertApplet: React.FC = () => {
  const appState = useRemoteApp();
  const [{ from, to }, onChangePair] = useState({ from: "USDC", to: "BTC" });

  return (
    <DangoRemoteProvider>
      <SimpleSwap pair={{ from, to }} onChangePair={onChangePair} appState={appState}>
        <SimpleSwap.Header />
        <SimpleSwap.Form />
        <SimpleSwap.Details />
        <SimpleSwap.Trigger />
      </SimpleSwap>
    </DangoRemoteProvider>
  );
};
