import { Comet38Client, HttpEndpoint } from "@cosmjs/tendermint-rpc";

export class Client {
  inner: Comet38Client;

  /**
   * Create a new CWD client for the given endpoint.
   */
  public static async connect(endpoint: string | HttpEndpoint): Promise<Client> {
    const inner = await Comet38Client.connect(endpoint);
    return new Client(inner);
  }

  /**
   * Do not use; use `Client.connect` instead.
   */
  private constructor(inner: Comet38Client) {
    this.inner = inner;
  }
}
