import {
  Button,
  createContext,
  JsonVisualizer,
  ResizerContainer,
  Tabs,
  useTheme,
} from "@left-curve/applets-kit";
import { useAccount, usePublicClient, useSigningClient, useSubmitTx } from "@left-curve/store";
import { Editor } from "@monaco-editor/react";
import { useMutation } from "@tanstack/react-query";

import { useState, type PropsWithChildren } from "react";
import { useApp } from "~/hooks/useApp";
import { m } from "~/paraglide/messages";

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

  const {
    isPending: queryIsPending,
    mutateAsync: query,
    data: queryResponse,
  } = useMutation({
    mutationFn: () =>
      client.queryWasmSmart(JSON.parse(queryMsg)).catch((e: any) => {
        return { error: e.details };
      }),
  });
  if (currentTab !== "query") return null;

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
                schemas: [executeSchema],
              });
            }}
            language="json"
            theme={theme === "dark" ? "vs-dark" : "vs-light"}
            width="100%"
            value={queryMsg}
            onChange={(v) => setQueryMsg(v ?? "")}
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
        {queryResponse ? (
          <div className="min-h-full p-4 bg-surface-primary-rice shadow-account-card  rounded-md  flex-1 w-full lg:w-auto overflow-auto">
            <JsonVisualizer json={JSON.stringify(queryResponse)} collapsed={1} />
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
  const { currentTab } = useMsgBuilder();
  const { toast } = useApp();
  const { data: signingClient } = useSigningClient();
  const { isConnected, account } = useAccount();
  const [executeMsg, setExecuteMsg] = useState<string>("");
  const { theme } = useTheme();

  const { isPending: executeIsPending, mutateAsync: execute } = useSubmitTx({
    toast: {
      success: () => toast.success({ description: "Message executed successfully" }),
      error: (error) => toast.error({ description: `Error executing message: ${error.message}` }),
    },
    mutation: {
      mutationFn: async () => {
        if (!signingClient || !account) {
          throw new Error("Signing client or account address is not available");
        }

        await signingClient.execute({ sender: account.address, execute: JSON.parse(executeMsg) });
      },
    },
  });
  if (currentTab !== "execute") return null;

  return (
    <div className="flex flex-col gap-4 w-full">
      <ResizerContainer
        layoutId="query-visualizer"
        className="flex flex-col lg:flex-row gap-4 w-full"
      >
        <div className="relative flex lg:h-[60vh] w-full flex-col overflow-hidden rounded-xl py-6 md:flex-1 bg-surface-playground pr-4 shadow-account-card">
          <Editor
            onMount={(_, monaco) => {
              monaco.languages.json.jsonDefaults.setDiagnosticsOptions({
                validate: true,
                schemas: [executeSchema],
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
        isLoading={executeIsPending}
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

const executeSchema = {
  uri: "",
  fileMatch: ["*"],
  schema: {
    $defs: {
      execute: {
        type: "object",
        properties: {
          contract: {
            type: "string",
            description: "The address of the contract to which the message will be sent.",
          },
          msg: {
            type: "object",
            description: "The message in JSON format that will be sent to the contract.",
            additionalProperties: true,
          },
          funds: {
            type: "array",
            description: "(Optional) Funds (coins) to be sent with the execution.",
            items: {
              type: "object",
              properties: {
                denom: { type: "string" },
                amount: { type: "string" },
              },
              required: ["denom", "amount"],
            },
          },
        },
        required: ["contract", "msg"],
      },
    },
    oneOf: [
      {
        $ref: "#/$defs/execute",
      },
      {
        type: "array",
        items: {
          $ref: "#/$defs/execute",
        },
      },
    ],
  },
};
