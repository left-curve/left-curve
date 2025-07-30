import { Button, twMerge, useTheme } from "@left-curve/applets-kit";
import { useAccount, useSigningClient, useSubmitTx } from "@left-curve/store";
import Editor from "@monaco-editor/react";
import { useState } from "react";
import { useApp } from "~/hooks/useApp";
import { m } from "~/paraglide/messages";

export const MsgBuilder: React.FC = () => {
  const [msg, setMsg] = useState<string>("");
  const { toast } = useApp();

  const { data: signingClient } = useSigningClient();
  const { isConnected, account } = useAccount();
  const { theme } = useTheme();

  const { isPending, mutateAsync: execute } = useSubmitTx({
    toast: {
      success: () => toast.success({ description: "Message executed successfully" }),
      error: (error) => toast.error({ description: `Error executing message: ${error.message}` }),
    },
    mutation: {
      mutationFn: async () => {
        if (!signingClient || !account) {
          throw new Error("Signing client or account address is not available");
        }
        
        await signingClient.execute({ sender: account.address, execute: JSON.parse(msg) });
      },
    },
  });

  return (
    <div className="w-full md:max-w-[80vw] mx-auto flex flex-col p-4 pt-6 gap-4 min-h-[100svh] md:min-h-[80svh]">
      <div
        className={twMerge(
          "relative flex h-[60vh] w-full flex-col overflow-hidden rounded-md py-6  shadow-sm md:flex-1",
          theme === "dark" ? "bg-[#1e1e1e]" : "bg-white",
        )}
      >
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
          value={msg}
          onChange={(v) => setMsg(v ?? "")}
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

      <div className="flex justify-end">
        <Button
          isLoading={isPending}
          isDisabled={!isConnected}
          onClick={() => execute()}
          className="w-full md:w-auto"
        >
          {m["devtools.msgBuilder.trigger"]()}
        </Button>
      </div>
    </div>
  );
};

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
