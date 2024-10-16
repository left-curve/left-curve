import { type Config, ConnectionStatus, type Connector } from "@leftcurve/types";

export type ConnectParameters = {
  chainId: string;
  username: string;
  connector: Connector;
  challenge?: string;
};

export type ConnectReturnType = void;

export type ConnectErrorType = Error;

export async function connect<config extends Config>(
  config: config,
  parameters: ConnectParameters,
): Promise<ConnectReturnType> {
  try {
    const { connector, ...rest } = parameters;

    config.setState((x) => ({ ...x, status: "connecting" }));
    connector.emitter.emit("message", { type: "connecting" });
    await connector.connect(rest);
  } catch (error) {
    config.setState((x) => ({
      ...x,
      status: x.connections.size > 0 ? ConnectionStatus.Connected : ConnectionStatus.Disconnected,
    }));
    throw error;
  }
}
