import { ethHashMessage, recoverPublicKey, ripemd160 } from "@leftcurve/crypto";
import { encodeBase64, encodeHex, encodeUtf8 } from "@leftcurve/encoding";
import { createBaseClient } from "@leftcurve/sdk";
import { getAccountsByKeyHash, getAccountsByUsername } from "@leftcurve/sdk/actions";
import { createConnector } from "./createConnector";

import type { Client, EIP1193Provider, Transport } from "@leftcurve/types";

import "@leftcurve/types/window";

type EIP1193ConnectorParameters = {
  id?: string;
  name?: string;
  icon?: string;
  provider?: () => EIP1193Provider | undefined;
};

export function eip1193(parameters: EIP1193ConnectorParameters = {}) {
  let _transport: Transport;
  let _username: string;
  let _client: Client;
  let _isAuthorized = false;

  const {
    id = "metamask",
    name = "Metamask",
    icon = "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIyMDAiIGhlaWdodD0iMjAwIiB2aWV3Qm94PSIwIDAgMjU2IDI0MCI+CiAgPHBhdGggZmlsbD0iI0UxNzcyNiIgZD0iTTI1MC4wNjYgMEwxNDAuMjE5IDgxLjI3OWwyMC40MjctNDcuOXoiLz4KICA8cGF0aCBmaWxsPSIjRTI3NjI1IiBkPSJtNi4xOTEuMDk2bDg5LjE4MSAzMy4yODlsMTkuMzk2IDQ4LjUyOHpNMjA1Ljg2IDE3Mi44NThsNDguNTUxLjkyNGwtMTYuOTY4IDU3LjY0MmwtNTkuMjQzLTE2LjMxMXptLTE1NS43MjEgMGwyNy41NTcgNDIuMjU1bC01OS4xNDMgMTYuMzEybC0xNi44NjUtNTcuNjQzeiIvPgogIDxwYXRoIGZpbGw9IiNFMjc2MjUiIGQ9Im0xMTIuMTMxIDY5LjU1MmwxLjk4NCA2NC4wODNsLTU5LjM3MS0yLjcwMWwxNi44ODgtMjUuNDc4bC4yMTQtLjI0NXptMzEuMTIzLS43MTVsNDAuOSAzNi4zNzZsLjIxMi4yNDRsMTYuODg4IDI1LjQ3OGwtNTkuMzU4IDIuN3pNNzkuNDM1IDE3My4wNDRsMzIuNDE4IDI1LjI1OWwtMzcuNjU4IDE4LjE4MXptOTcuMTM2LS4wMDRsNS4xMzEgNDMuNDQ1bC0zNy41NTMtMTguMTg0eiIvPgogIDxwYXRoIGZpbGw9IiNENUJGQjIiIGQ9Im0xNDQuOTc4IDE5NS45MjJsMzguMTA3IDE4LjQ1MmwtMzUuNDQ3IDE2Ljg0NmwuMzY4LTExLjEzNHptLTMzLjk2Ny4wMDhsLTIuOTA5IDIzLjk3NGwuMjM5IDExLjMwM2wtMzUuNTMtMTYuODMzeiIvPgogIDxwYXRoIGZpbGw9IiMyMzM0NDciIGQ9Im0xMDAuMDA3IDE0MS45OTlsOS45NTggMjAuOTI4bC0zMy45MDMtOS45MzJ6bTU1Ljk4NS4wMDJsMjQuMDU4IDEwLjk5NGwtMzQuMDE0IDkuOTI5eiIvPgogIDxwYXRoIGZpbGw9IiNDQzYyMjgiIGQ9Im04Mi4wMjYgMTcyLjgzbC01LjQ4IDQ1LjA0bC0yOS4zNzMtNDQuMDU1em05MS45NS4wMDFsMzQuODU0Ljk4NGwtMjkuNDgzIDQ0LjA1N3ptMjguMTM2LTQ0LjQ0NGwtMjUuMzY1IDI1Ljg1MWwtMTkuNTU3LTguOTM3bC05LjM2MyAxOS42ODRsLTYuMTM4LTMzLjg0OXptLTE0OC4yMzcgMGw2MC40MzUgMi43NDlsLTYuMTM5IDMzLjg0OWwtOS4zNjUtMTkuNjgxbC0xOS40NTMgOC45MzV6Ii8+CiAgPHBhdGggZmlsbD0iI0UyNzUyNSIgZD0ibTUyLjE2NiAxMjMuMDgybDI4LjY5OCAyOS4xMjFsLjk5NCAyOC43NDl6bTE1MS42OTctLjA1MmwtMjkuNzQ2IDU3Ljk3M2wxLjEyLTI4Ljh6bS05MC45NTYgMS44MjZsMS4xNTUgNy4yN2wyLjg1NCAxOC4xMTFsLTEuODM1IDU1LjYyNWwtOC42NzUtNDQuNjg1bC0uMDAzLS40NjJ6bTMwLjE3MS0uMTAxbDYuNTIxIDM1Ljk2bC0uMDAzLjQ2MmwtOC42OTcgNDQuNzk3bC0uMzQ0LTExLjIwNWwtMS4zNTctNDQuODYyeiIvPgogIDxwYXRoIGZpbGw9IiNGNTg0MUYiIGQ9Im0xNzcuNzg4IDE1MS4wNDZsLS45NzEgMjQuOTc4bC0zMC4yNzQgMjMuNTg3bC02LjEyLTQuMzI0bDYuODYtMzUuMzM1em0tOTkuNDcxIDBsMzAuMzk5IDguOTA2bDYuODYgMzUuMzM1bC02LjEyIDQuMzI0bC0zMC4yNzUtMjMuNTg5eiIvPgogIDxwYXRoIGZpbGw9IiNDMEFDOUQiIGQ9Im02Ny4wMTggMjA4Ljg1OGwzOC43MzIgMTguMzUybC0uMTY0LTcuODM3bDMuMjQxLTIuODQ1aDM4LjMzNGwzLjM1OCAyLjgzNWwtLjI0OCA3LjgzMWwzOC40ODctMTguMjlsLTE4LjcyOCAxNS40NzZsLTIyLjY0NSAxNS41NTNoLTM4Ljg2OWwtMjIuNjMtMTUuNjE3eiIvPgogIDxwYXRoIGZpbGw9IiMxNjE2MTYiIGQ9Im0xNDIuMjA0IDE5My40NzlsNS40NzYgMy44NjlsMy4yMDkgMjUuNjA0bC00LjY0NC0zLjkyMWgtMzYuNDc2bC00LjU1NiA0bDMuMTA0LTI1LjY4MWw1LjQ3OC0zLjg3MXoiLz4KICA8cGF0aCBmaWxsPSIjNzYzRTFBIiBkPSJNMjQyLjgxNCAyLjI1TDI1NiA0MS44MDdsLTguMjM1IDM5Ljk5N2w1Ljg2NCA0LjUyM2wtNy45MzUgNi4wNTRsNS45NjQgNC42MDZsLTcuODk3IDcuMTkxbDQuODQ4IDMuNTExbC0xMi44NjYgMTUuMDI2bC01Mi43Ny0xNS4zNjVsLS40NTctLjI0NWwtMzguMDI3LTMyLjA3OHptLTIyOS42MjggMGw5OC4zMjYgNzIuNzc3bC0zOC4wMjggMzIuMDc4bC0uNDU3LjI0NWwtNTIuNzcgMTUuMzY1bC0xMi44NjYtMTUuMDI2bDQuODQ0LTMuNTA4bC03Ljg5Mi03LjE5NGw1Ljk1Mi00LjYwMWwtOC4wNTQtNi4wNzFsNi4wODUtNC41MjZMMCA0MS44MDl6Ii8+CiAgPHBhdGggZmlsbD0iI0Y1ODQxRiIgZD0ibTE4MC4zOTIgMTAzLjk5bDU1LjkxMyAxNi4yNzlsMTguMTY1IDU1Ljk4NmgtNDcuOTI0bC0zMy4wMi40MTZsMjQuMDE0LTQ2LjgwOHptLTEwNC43ODQgMGwtMTcuMTUxIDI1Ljg3M2wyNC4wMTcgNDYuODA4bC0zMy4wMDUtLjQxNkgxLjYzMWwxOC4wNjMtNTUuOTg1em04Ny43NzYtNzAuODc4bC0xNS42MzkgNDIuMjM5bC0zLjMxOSA1Ny4wNmwtMS4yNyAxNy44ODVsLS4xMDEgNDUuNjg4aC0zMC4xMTFsLS4wOTgtNDUuNjAybC0xLjI3NC0xNy45ODZsLTMuMzItNTcuMDQ1bC0xNS42MzctNDIuMjM5eiIvPgo8L3N2Zz4=",
    provider: _provider_ = () => window.ethereum,
  } = parameters;

  return createConnector<EIP1193Provider>(({ transports, emitter }) => {
    return {
      id,
      name,
      icon,
      type: "eip1193",
      async connect({ username, chainId, challenge }) {
        _username = username;
        _transport = transports[chainId];
        await this.getClient();
        const accounts = await this.getAccounts();
        const provider = await this.getProvider();
        const [controllerAddress] = await provider.request({ method: "eth_requestAccounts" });
        if (challenge) {
          const signature = await provider.request({
            method: "personal_sign",
            params: [challenge, controllerAddress],
          });
          const hashMessage = ethHashMessage(challenge);
          const publicKey = await recoverPublicKey(hashMessage, signature);

          const keyHash = encodeHex(ripemd160(publicKey)).toUpperCase();
          const usernames = await getAccountsByKeyHash(_client, { hash: keyHash });
          if (!usernames.includes(username)) throw new Error("Not authorized");
          _isAuthorized = true;
        }
        emitter.emit("connect", { accounts, chainId, username });
      },
      async disconnect() {
        _isAuthorized = false;
        emitter.emit("disconnect");
      },
      async getClient() {
        if (!_client) _client = createBaseClient({ transport: _transport });
        return _client;
      },
      async getProvider() {
        const provider = _provider_();
        if (!provider) throw new Error(`${name} not detected`);
        return provider;
      },
      async getAccounts() {
        const accounts = await getAccountsByUsername(_client, { username: _username });
        return Object.entries(accounts).map(([index, info]) => ({
          id: `${_username}/account/${Number(index)}`,
          index: Number(index),
          username: _username,
          ...info,
        }));
      },
      async isAuthorized() {
        return _isAuthorized;
      },
      async requestSignature(typedData) {
        const provider = await this.getProvider();
        const [controllerAddress] = await provider.request({ method: "eth_requestAccounts" });
        const signature = await provider.request({
          method: "eth_signTypedData_v4",
          params: [controllerAddress, JSON.stringify(typedData)],
        });
        const credential = encodeUtf8(JSON.stringify({ signature, typed_data: typedData }));
        return { walletEvm: encodeBase64(credential) };
      },
    };
  });
}
