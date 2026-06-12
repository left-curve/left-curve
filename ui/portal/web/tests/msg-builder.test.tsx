import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { Suspense } from "react";

import type React from "react";

import { resetAppletsKitMocks, setAppletsKitUseTheme } from "./mocks/applets-kit";
import { MsgBuilder } from "../src/components/devtools/MsgBuilder";

const msgBuilderMocks = vi.hoisted(() => ({
  configure: vi.fn(),
  diagnosticsOptions: [] as Array<{
    schemas: Array<{
      schema: {
        $defs: Record<string, unknown>;
      };
    }>;
  }>,
  execute: vi.fn(),
  instantiate: vi.fn(),
  migrate: vi.fn(),
  queryApp: vi.fn(),
  storeCode: vi.fn(),
  submitMutationFn: undefined as undefined | (() => Promise<unknown>),
  transfer: vi.fn(),
  upgrade: vi.fn(),
  useAccount: vi.fn(),
  useAppConfig: vi.fn(),
  useBalances: vi.fn(),
  usePublicClient: vi.fn(),
  useSigningClient: vi.fn(),
  useSubmitTx: vi.fn(),
}));

type DiagnosticsOptions = (typeof msgBuilderMocks.diagnosticsOptions)[number];

vi.mock("@left-curve/sdk/actions", () => ({
  configure: msgBuilderMocks.configure,
  upgrade: msgBuilderMocks.upgrade,
}));

vi.mock("framer-motion", () => ({
  motion: {
    div: ({
      animate: _animate,
      children,
      exit: _exit,
      initial: _initial,
      layout: _layout,
      layoutId: _layoutId,
      layoutRoot: _layoutRoot,
      transition: _transition,
      ...props
    }: React.HTMLAttributes<HTMLDivElement> & {
      animate?: unknown;
      exit?: unknown;
      initial?: unknown;
      layout?: unknown;
      layoutId?: string;
      layoutRoot?: unknown;
      transition?: unknown;
    }) => <div {...props}>{children}</div>,
  },
}));

vi.mock("@monaco-editor/react", async () => {
  const React = await import("react");

  return {
    Editor: ({
      height: _height,
      language,
      onChange,
      onMount,
      options: _options,
      theme,
      value,
      width: _width,
    }: {
      height?: string;
      language: string;
      onChange?: (value?: string) => void;
      onMount?: (editor: unknown, monaco: unknown) => void;
      options?: unknown;
      theme?: string;
      value: string;
      width?: string;
    }) => {
      React.useEffect(() => {
        onMount?.(
          {},
          {
            languages: {
              json: {
                jsonDefaults: {
                  setDiagnosticsOptions: (options: DiagnosticsOptions) => {
                    msgBuilderMocks.diagnosticsOptions.push(options);
                  },
                },
              },
            },
          },
        );
      }, [onMount]);

      return (
        <textarea
          aria-label={`${language} editor`}
          data-theme={theme}
          onChange={(event) => onChange?.(event.target.value)}
          value={value}
        />
      );
    },
  };
});

vi.mock("@tanstack/react-query", async () => {
  const React = await import("react");

  return {
    useMutation: <Result,>({ mutationFn }: { mutationFn: () => Promise<Result> }) => {
      const [data, setData] = React.useState<Result | undefined>();
      const [isPending, setIsPending] = React.useState(false);

      return {
        data,
        isPending,
        mutateAsync: async () => {
          setIsPending(true);
          try {
            const result = await mutationFn();
            setData(result);
            return result;
          } finally {
            setIsPending(false);
          }
        },
      };
    },
  };
});

vi.mock("@microlink/react-json-view", () => ({
  default: ({ src }: { src: unknown }) => (
    <pre data-testid="json-visualizer">{JSON.stringify(src)}</pre>
  ),
}));

vi.mock("@left-curve/store", () => ({
  useAccount: msgBuilderMocks.useAccount,
  useAppConfig: msgBuilderMocks.useAppConfig,
  useBalances: msgBuilderMocks.useBalances,
  usePublicClient: msgBuilderMocks.usePublicClient,
  useSigningClient: msgBuilderMocks.useSigningClient,
  useSubmitTx: msgBuilderMocks.useSubmitTx,
}));

const account = {
  address: "0x646576746f6f6c75736572000000000000000000",
};

function getCapturedSubmitMutation() {
  if (!msgBuilderMocks.submitMutationFn) {
    throw new Error("Expected message builder submit mutation to be captured");
  }
  return msgBuilderMocks.submitMutationFn;
}

function renderMsgBuilder() {
  return render(
    <Suspense fallback={null}>
      <MsgBuilder>
        <MsgBuilder.QueryMsg />
        <MsgBuilder.ExecuteMsg />
      </MsgBuilder>
    </Suspense>,
  );
}

function findJsonVisualizer() {
  return screen.findByTestId("json-visualizer", undefined, { timeout: 5000 });
}

describe("devtools message builder", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    setAppletsKitUseTheme({
      hasLoaded: true,
      setThemeSchema: vi.fn(),
      theme: "light",
      themeSchema: "light",
    });
    msgBuilderMocks.diagnosticsOptions.length = 0;
    class ResizeObserverMock {
      disconnect = vi.fn();
      observe = vi.fn();
      unobserve = vi.fn();
    }

    Object.defineProperty(globalThis, "ResizeObserver", {
      configurable: true,
      value: ResizeObserverMock,
    });
    msgBuilderMocks.queryApp.mockResolvedValue({
      height: 42,
      status: "ok",
    });
    msgBuilderMocks.execute.mockResolvedValue({ txHash: "0x65786563" });
    msgBuilderMocks.instantiate.mockResolvedValue({ txHash: "0x696e7374" });
    msgBuilderMocks.migrate.mockResolvedValue({ txHash: "0x6d6967" });
    msgBuilderMocks.storeCode.mockResolvedValue({ codeHash: "0x636f6465" });
    msgBuilderMocks.submitMutationFn = undefined;
    msgBuilderMocks.transfer.mockResolvedValue({ txHash: "0x7472616e" });
    msgBuilderMocks.upgrade.mockResolvedValue({ txHash: "0x757067" });
    msgBuilderMocks.configure.mockResolvedValue({ txHash: "0x636667" });
    msgBuilderMocks.useAccount.mockReturnValue({
      account,
      isConnected: true,
    });
    msgBuilderMocks.useAppConfig.mockReturnValue({
      data: {
        addresses: {
          bank: "0x62616e6b00000000000000000000000000000000",
          dex: "0x6465780000000000000000000000000000000000",
          "0xraw": "0x7261770000000000000000000000000000000000",
        },
      },
    });
    msgBuilderMocks.useBalances.mockReturnValue({
      data: {
        "bridge/btc": "3",
        "bridge/usdc": "1200",
      },
    });
    msgBuilderMocks.usePublicClient.mockReturnValue({
      queryApp: msgBuilderMocks.queryApp,
    });
    msgBuilderMocks.useSigningClient.mockReturnValue({
      data: {
        execute: msgBuilderMocks.execute,
        instantiate: msgBuilderMocks.instantiate,
        migrate: msgBuilderMocks.migrate,
        storeCode: msgBuilderMocks.storeCode,
        transfer: msgBuilderMocks.transfer,
      },
    });
    msgBuilderMocks.useSubmitTx.mockImplementation(
      ({
        mutation,
      }: {
        mutation: {
          mutationFn: () => Promise<unknown>;
        };
      }) => {
        msgBuilderMocks.submitMutationFn = mutation.mutationFn;

        return {
          isPending: false,
          mutateAsync: async () => {
            try {
              return await mutation.mutationFn();
            } catch {
              return undefined;
            }
          },
        };
      },
    );
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("submits query JSON through the public client and visualizes the response", async () => {
    renderMsgBuilder();

    fireEvent.change(screen.getByLabelText("json editor"), {
      target: {
        value: '{"status":{}}',
      },
    });
    fireEvent.click(screen.getByRole("button", { name: m["devtools.msgBuilder.query"]() }));

    await waitFor(() =>
      expect(msgBuilderMocks.queryApp).toHaveBeenCalledWith({
        query: {
          status: {},
        },
      }),
    );
    expect(await findJsonVisualizer()).toHaveTextContent(
      JSON.stringify({
        response: {
          height: 42,
          status: "ok",
        },
      }),
    );

    const addressDefinition = msgBuilderMocks.diagnosticsOptions.at(-1)?.schemas[0].schema.$defs
      .Address as {
      anyOf: Array<{
        enum?: string[];
      }>;
    };

    expect(addressDefinition.anyOf[0].enum).toEqual([
      "0x62616e6b00000000000000000000000000000000",
      "0x6465780000000000000000000000000000000000",
    ]);
  });

  it("visualizes backend query error details without submitting an execute transaction", async () => {
    msgBuilderMocks.queryApp.mockRejectedValueOnce({
      details: {
        message: "contract query failed",
      },
    });

    renderMsgBuilder();

    fireEvent.change(screen.getByLabelText("json editor"), {
      target: {
        value: '{"status":{}}',
      },
    });
    fireEvent.click(screen.getByRole("button", { name: m["devtools.msgBuilder.query"]() }));

    expect(await findJsonVisualizer()).toHaveTextContent(
      JSON.stringify({
        error: {
          message: "contract query failed",
        },
      }),
    );
    expect(msgBuilderMocks.queryApp).toHaveBeenCalledWith({
      query: {
        status: {},
      },
    });
    expect(msgBuilderMocks.execute).not.toHaveBeenCalled();
    expect(msgBuilderMocks.transfer).not.toHaveBeenCalled();
  });

  it("builds execute funds schema from wallet balances and disables execution when disconnected", () => {
    renderMsgBuilder();

    fireEvent.click(screen.getByRole("button", { name: "execute" }));

    expect(msgBuilderMocks.useBalances).toHaveBeenCalledWith({
      address: account.address,
    });
    expect(screen.getByRole("button", { name: m["devtools.msgBuilder.execute"]() })).toBeEnabled();

    const fundsDefinition = msgBuilderMocks.diagnosticsOptions.at(-1)?.schemas[0].schema.$defs
      .Funds as {
      properties: Record<string, { description: string }>;
    };

    expect(fundsDefinition.properties).toEqual({
      "bridge/btc": {
        description: "Available balance: 3",
        type: "string",
      },
      "bridge/usdc": {
        description: "Available balance: 1200",
        type: "string",
      },
    });

    cleanup();
    msgBuilderMocks.useAccount.mockReturnValue({
      account: undefined,
      isConnected: false,
    });

    renderMsgBuilder();

    fireEvent.click(screen.getByRole("button", { name: "execute" }));

    expect(screen.getByRole("button", { name: m["devtools.msgBuilder.execute"]() })).toBeDisabled();
  });

  it("does not route malformed execute JSON to signing clients", async () => {
    renderMsgBuilder();

    fireEvent.click(screen.getByRole("button", { name: "execute" }));

    fireEvent.change(screen.getByLabelText("json editor"), {
      target: {
        value: '{"transfer":',
      },
    });
    fireEvent.click(screen.getByRole("button", { name: m["devtools.msgBuilder.execute"]() }));

    await waitFor(() => {
      expect(msgBuilderMocks.execute).not.toHaveBeenCalled();
      expect(msgBuilderMocks.instantiate).not.toHaveBeenCalled();
      expect(msgBuilderMocks.migrate).not.toHaveBeenCalled();
      expect(msgBuilderMocks.storeCode).not.toHaveBeenCalled();
      expect(msgBuilderMocks.transfer).not.toHaveBeenCalled();
      expect(msgBuilderMocks.upgrade).not.toHaveBeenCalled();
      expect(msgBuilderMocks.configure).not.toHaveBeenCalled();
    });
  });

  it("does not route execute messages before the signing client is available", async () => {
    msgBuilderMocks.useSigningClient.mockReturnValue({
      data: undefined,
    });

    renderMsgBuilder();

    fireEvent.click(screen.getByRole("button", { name: "execute" }));
    fireEvent.change(screen.getByLabelText("json editor"), {
      target: {
        value:
          '{"execute":{"contract":"0x636f6e7472616374000000000000000000000000","msg":{"ping":{}}}}',
      },
    });
    fireEvent.click(screen.getByRole("button", { name: m["devtools.msgBuilder.execute"]() }));

    await waitFor(() => {
      expect(msgBuilderMocks.execute).not.toHaveBeenCalled();
      expect(msgBuilderMocks.instantiate).not.toHaveBeenCalled();
      expect(msgBuilderMocks.migrate).not.toHaveBeenCalled();
      expect(msgBuilderMocks.storeCode).not.toHaveBeenCalled();
      expect(msgBuilderMocks.transfer).not.toHaveBeenCalled();
      expect(msgBuilderMocks.upgrade).not.toHaveBeenCalled();
      expect(msgBuilderMocks.configure).not.toHaveBeenCalled();
    });
  });

  it("propagates backend transfer failures after building the transfer payload", async () => {
    msgBuilderMocks.transfer.mockRejectedValueOnce(new Error("transfer rejected"));
    renderMsgBuilder();

    fireEvent.click(screen.getByRole("button", { name: "execute" }));
    fireEvent.change(screen.getByLabelText("json editor"), {
      target: {
        value: '{"transfer":{"0x726563697069656e740000000000000000000000":{"bridge/usdc":"5"}}}',
      },
    });

    await expect(getCapturedSubmitMutation()()).rejects.toThrow("transfer rejected");

    expect(msgBuilderMocks.transfer).toHaveBeenCalledWith({
      sender: account.address,
      transfer: {
        "0x726563697069656e740000000000000000000000": {
          "bridge/usdc": "5",
        },
      },
    });
    expect(msgBuilderMocks.execute).not.toHaveBeenCalled();
    expect(msgBuilderMocks.instantiate).not.toHaveBeenCalled();
    expect(msgBuilderMocks.migrate).not.toHaveBeenCalled();
    expect(msgBuilderMocks.storeCode).not.toHaveBeenCalled();
    expect(msgBuilderMocks.upgrade).not.toHaveBeenCalled();
    expect(msgBuilderMocks.configure).not.toHaveBeenCalled();
  });

  it("routes execute, transfer, upload, upgrade, and configure messages to the expected clients", async () => {
    renderMsgBuilder();

    fireEvent.click(screen.getByRole("button", { name: "execute" }));

    const editor = screen.getByLabelText("json editor");
    const executeButton = screen.getByRole("button", { name: m["devtools.msgBuilder.execute"]() });

    fireEvent.change(editor, {
      target: {
        value:
          '{"execute":{"contract":"0x636f6e7472616374000000000000000000000000","msg":{"ping":{}},"funds":{"bridge/usdc":"7"}}}',
      },
    });
    fireEvent.click(executeButton);

    await waitFor(() =>
      expect(msgBuilderMocks.execute).toHaveBeenCalledWith({
        execute: {
          contract: "0x636f6e7472616374000000000000000000000000",
          funds: {
            "bridge/usdc": "7",
          },
          msg: {
            ping: {},
          },
        },
        sender: account.address,
      }),
    );

    fireEvent.change(editor, {
      target: {
        value: '{"transfer":{"0x726563697069656e740000000000000000000000":{"bridge/usdc":"5"}}}',
      },
    });
    fireEvent.click(executeButton);

    await waitFor(() =>
      expect(msgBuilderMocks.transfer).toHaveBeenCalledWith({
        sender: account.address,
        transfer: {
          "0x726563697069656e740000000000000000000000": {
            "bridge/usdc": "5",
          },
        },
      }),
    );

    fireEvent.change(editor, {
      target: {
        value: '{"upload":{"code":"aGVsbG8="}}',
      },
    });
    fireEvent.click(executeButton);

    await waitFor(() =>
      expect(msgBuilderMocks.storeCode).toHaveBeenCalledWith({
        code: "aGVsbG8=",
        sender: account.address,
      }),
    );

    fireEvent.change(editor, {
      target: {
        value: '{"upgrade":{"height":120,"cargoVersion":"1.2.3","gitTag":"v1.2.3"}}',
      },
    });
    fireEvent.click(executeButton);

    await waitFor(() =>
      expect(msgBuilderMocks.upgrade).toHaveBeenCalledWith(
        expect.objectContaining({
          execute: msgBuilderMocks.execute,
          instantiate: msgBuilderMocks.instantiate,
          migrate: msgBuilderMocks.migrate,
          storeCode: msgBuilderMocks.storeCode,
          transfer: msgBuilderMocks.transfer,
        }),
        expect.objectContaining({
          sender: account.address,
          height: 120,
          cargoVersion: "1.2.3",
          gitTag: "v1.2.3",
        }),
      ),
    );

    fireEvent.change(editor, {
      target: {
        value: '{"configure":{"newCfg":{"foo":"bar"},"newAppCfg":{"baz":1}}}',
      },
    });
    fireEvent.click(executeButton);

    await waitFor(() =>
      expect(msgBuilderMocks.configure).toHaveBeenCalledWith(
        expect.objectContaining({
          execute: msgBuilderMocks.execute,
          instantiate: msgBuilderMocks.instantiate,
          migrate: msgBuilderMocks.migrate,
          storeCode: msgBuilderMocks.storeCode,
          transfer: msgBuilderMocks.transfer,
        }),
        expect.objectContaining({
          sender: account.address,
          newCfg: {
            foo: "bar",
          },
          newAppCfg: {
            baz: 1,
          },
        }),
      ),
    );
  });

  it("routes batched execute messages to the signing client without changing their order", async () => {
    renderMsgBuilder();

    fireEvent.click(screen.getByRole("button", { name: "execute" }));

    const editor = screen.getByLabelText("json editor");
    const executeButton = screen.getByRole("button", { name: m["devtools.msgBuilder.execute"]() });

    fireEvent.change(editor, {
      target: {
        value:
          '{"execute":[{"contract":"0x66697273742d636f6e7472616374000000000000","msg":{"open":{}},"funds":{"bridge/usdc":"1"}},{"contract":"0x7365636f6e642d636f6e74726163740000000000","msg":{"close":{"id":"position-7"}}}]}',
      },
    });
    fireEvent.click(executeButton);

    await waitFor(() =>
      expect(msgBuilderMocks.execute).toHaveBeenCalledWith({
        execute: [
          {
            contract: "0x66697273742d636f6e7472616374000000000000",
            funds: {
              "bridge/usdc": "1",
            },
            msg: {
              open: {},
            },
          },
          {
            contract: "0x7365636f6e642d636f6e74726163740000000000",
            msg: {
              close: {
                id: "position-7",
              },
            },
          },
        ],
        sender: account.address,
      }),
    );
    expect(msgBuilderMocks.execute).toHaveBeenCalledOnce();
    expect(msgBuilderMocks.transfer).not.toHaveBeenCalled();
    expect(msgBuilderMocks.instantiate).not.toHaveBeenCalled();
    expect(msgBuilderMocks.migrate).not.toHaveBeenCalled();
  });

  it("routes instantiate and migrate messages to the signing client", async () => {
    renderMsgBuilder();

    fireEvent.click(screen.getByRole("button", { name: "execute" }));

    const editor = screen.getByLabelText("json editor");
    const executeButton = screen.getByRole("button", { name: m["devtools.msgBuilder.execute"]() });

    fireEvent.change(editor, {
      target: {
        value:
          '{"instantiate":{"codeHash":"0x636f646568617368000000000000000000000000","label":"counter","msg":{"count":1},"funds":{"bridge/usdc":"9"}}}',
      },
    });
    fireEvent.click(executeButton);

    await waitFor(() =>
      expect(msgBuilderMocks.instantiate).toHaveBeenCalledWith({
        codeHash: "0x636f646568617368000000000000000000000000",
        funds: {
          "bridge/usdc": "9",
        },
        label: "counter",
        msg: {
          count: 1,
        },
        sender: account.address,
      }),
    );

    fireEvent.change(editor, {
      target: {
        value:
          '{"migrate":{"contract":"0x636f6e7472616374000000000000000000000000","newCodeHash":"0x6e6577636f646500000000000000000000000000","msg":{"version":2}}}',
      },
    });
    fireEvent.click(executeButton);

    await waitFor(() =>
      expect(msgBuilderMocks.migrate).toHaveBeenCalledWith({
        contract: "0x636f6e7472616374000000000000000000000000",
        msg: {
          version: 2,
        },
        newCodeHash: "0x6e6577636f646500000000000000000000000000",
        sender: account.address,
      }),
    );
  });
});
