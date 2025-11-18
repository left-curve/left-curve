import {
  Button,
  createContext,
  JsonVisualizer,
  ResizerContainer,
  Tabs,
  useApp,
  useTheme,
} from "@left-curve/applets-kit";
import {
  useAccount,
  useAppConfig,
  useBalances,
  usePublicClient,
  useSigningClient,
  useSubmitTx,
} from "@left-curve/store";
import { Editor } from "@monaco-editor/react";
import { useMutation } from "@tanstack/react-query";
import { tryCatch } from "@left-curve/dango/utils";
import { upgrade, configure } from "@left-curve/dango/actions";

import { useState, type PropsWithChildren } from "react";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import querySchema from "./querySchema.json";
import executeSchema from "./executeSchema.json";

type MsgBuilderProps = {
  currentTab: "execute" | "query";
};

const [MsgBuilderProvider, useMsgBuilder] = createContext<MsgBuilderProps>({
  name: "MsgBuilderContext",
});

const MsgBuilderContainer: React.FC<PropsWithChildren> = ({ children }) => {
  const [currentTab, setCurrentTab] = useState<"execute" | "query">("query");
  return (
    <MsgBuilderProvider value={{ currentTab }}>
      <div className="w-full mx-auto flex flex-col gap-6 md:max-w-[76rem] p-4">
        <div className="flex relative">
          <Tabs
            color="green"
            layoutId="tabs-msg-builder"
            selectedTab={currentTab}
            keys={["query", "execute"]}
            onTabChange={(tab) => setCurrentTab(tab as "execute" | "query")}
          />
        </div>
        <ResizerContainer layoutId="msg-builder">{children}</ResizerContainer>
      </div>
    </MsgBuilderProvider>
  );
};

const QueryMsg: React.FC = () => {
  const { currentTab } = useMsgBuilder();

  const client = usePublicClient();
  const [queryMsg, setQueryMsg] = useState<string>("");
  const { theme } = useTheme();
  const { data: config } = useAppConfig();

  const {
    isPending: queryIsPending,
    mutateAsync: query,
    data: queryResponse,
  } = useMutation({
    mutationFn: () =>
      client
        .queryApp({ query: JSON.parse(queryMsg) })
        .then((r) => ({ response: r }))
        .catch((e: any) => {
          return { error: e.details };
        }),
  });
  if (currentTab !== "query" || !config) return null;

  const addresses = Object.fromEntries(
    Object.entries(config.addresses).filter(([key]) => !key.includes("0x")),
  );

  querySchema.$defs.Address = {
    anyOf: [
      {
        type: "string",
        enum: Object.values(addresses),
        enumDescriptions: Object.keys(addresses).map((k) =>
          m["explorer.contracts.contractDescription"]({ contract: k }),
        ),
      },
      {
        type: "string",
      },
    ],
    description: "The address of the contract to which the query will be sent.",
  } as unknown as { type: string; description: string };

  return (
    <div className="flex flex-col gap-4 w-full">
      <ResizerContainer
        layoutId="query-visualizer"
        className="flex flex-col lg:flex-row gap-4 w-full"
      >
        <div className="relative flex min-h-[60vh] flex-col overflow-hidden rounded-xl py-6 bg-surface-playground pr-4 shadow-account-card flex-1 w-full lg:w-auto">
          <Editor
            onMount={(_, monaco) => {
              monaco.languages.json.jsonDefaults.setDiagnosticsOptions({
                validate: true,
                schemas: [
                  {
                    uri: "",
                    fileMatch: ["*"],
                    schema: querySchema,
                  },
                ],
              });
            }}
            language="json"
            theme={theme === "dark" ? "vs-dark" : "vs-light"}
            width="100%"
            height="60vh"
            value={queryMsg}
            onChange={(v) => setQueryMsg(v ?? "")}
            options={{
              automaticLayout: true,
              scrollbar: {
                verticalScrollbarSize: 0,
              },
              suggest: {
                showStatusBar: true,
              },
              minimap: {
                enabled: false,
              },
              formatOnPaste: true,
              formatOnType: true,
            }}
          />
        </div>
        {queryResponse ? (
          <div className="min-h-[60vh] lg:min-h-full p-4 bg-surface-tertiary-rice shadow-account-card  rounded-xl  flex-1 w-full lg:w-auto overflow-auto">
            <div className="overflow-hidden rounded-lg p-2 bg-[#453d39] h-[61vh] overflow-y-scroll scrollbar-none">
              <JsonVisualizer json={queryResponse} collapsed={1} />
            </div>
          </div>
        ) : null}
      </ResizerContainer>
      <Button isLoading={queryIsPending} onClick={() => query()} className="w-full md:w-auto">
        {m["devtools.msgBuilder.query"]()}
      </Button>
    </div>
  );
};

const ExecuteMsg: React.FC = () => {
  const { toast } = useApp();
  const { currentTab } = useMsgBuilder();
  const { data: signingClient } = useSigningClient();
  const { isConnected, account } = useAccount();
  const { data: balances = {} } = useBalances({ address: account?.address });
  const [executeMsg, setExecuteMsg] = useState<string>("");
  const { theme } = useTheme();

  const { isPending, mutateAsync: execute } = useSubmitTx({
    toast: {
      error: (error) => toast.error({ title: m["common.error"](), description: String(error) }),
    },
    mutation: {
      mutationFn: async () => {
        if (!signingClient || !account) {
          throw new Error("Signing client or account address is not available");
        }

        const { data: message } = tryCatch(() => JSON.parse(executeMsg));
        if (!message) throw new Error("Invalid execute message");

        if ("execute" in message) {
          return await signingClient.execute({ sender: account.address, execute: message.execute });
        }

        if ("migrate" in message) {
          return await signingClient.migrate({ sender: account.address, ...message.migrate });
        }

        if ("instantiate" in message) {
          return await signingClient.instantiate({
            sender: account.address,
            ...message.instantiate,
          });
        }

        if ("transfer" in message) {
          return await signingClient.transfer({ sender: account.address, ...message.transfer });
        }

        if ("upload" in message) {
          return await signingClient.storeCode({
            sender: account.address,
            code: message.upload.code,
          });
        }

        if ("upgrade" in message) {
          return await upgrade(signingClient, {
            sender: account.address,
            ...message.upgrade,
          });
        }

        if ("configure" in message) {
          return await configure(signingClient, {
            sender: account.address,
            ...message.configure,
          });
        }

        throw new Error("Unsupported message type");
      },
    },
  });

  if (currentTab !== "execute") return null;

  executeSchema.$defs.Funds = {
    type: "object",
    description: "(Optional) Funds (coins) to be sent with the execution.",
    properties: Object.fromEntries(
      Object.entries(balances).map(([denom, balance]) => [
        denom,
        {
          type: "string",
          description: `Available balance: ${balance}`,
        },
      ]),
    ),
  };
  return (
    <div className="flex flex-col gap-4 w-full">
      <ResizerContainer
        layoutId="query-visualizer"
        className="flex flex-col lg:flex-row gap-4 w-full"
      >
        <div className="relative flex min-h-[60vh] w-full flex-col overflow-hidden rounded-xl py-6 md:flex-1 bg-surface-playground pr-4 shadow-account-card">
          <Editor
            onMount={(_, monaco) => {
              monaco.languages.json.jsonDefaults.setDiagnosticsOptions({
                validate: true,
                schemas: [
                  {
                    uri: "",
                    fileMatch: ["*"],
                    schema: executeSchema,
                  },
                ],
              });
            }}
            language="json"
            theme={theme === "dark" ? "vs-dark" : "vs-light"}
            width="100%"
            height="60vh"
            value={executeMsg}
            onChange={(v) => setExecuteMsg(v ?? "")}
            options={{
              automaticLayout: true,
              scrollbar: {
                verticalScrollbarSize: 0,
              },
              minimap: {
                enabled: false,
              },
              formatOnPaste: true,
              formatOnType: true,
            }}
          />
        </div>
      </ResizerContainer>
      <Button
        isLoading={isPending}
        isDisabled={!isConnected}
        onClick={() => execute()}
        className="w-full"
      >
        {m["devtools.msgBuilder.execute"]()}
      </Button>
    </div>
  );
};

export const MsgBuilder = Object.assign(MsgBuilderContainer, {
  QueryMsg,
  ExecuteMsg,
});
