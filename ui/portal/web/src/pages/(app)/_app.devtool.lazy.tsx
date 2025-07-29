import { Button, useTheme } from "@left-curve/applets-kit";
import { useAccount, useSigningClient, useSubmitTx } from "@left-curve/store";
import Editor, { type OnMount } from "@monaco-editor/react";
import { createLazyFileRoute, useNavigate } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";

export const Route = createLazyFileRoute("/(app)/_app/devtool")({
  component: DevtoolApplet,
});

function DevtoolApplet() {
  const [msg, setMsg] = useState<string>("");
  const editorRef = useRef<any>(null);

  const { data: signingClient } = useSigningClient();
  const { account } = useAccount();
  const { theme } = useTheme();
  const navigate = useNavigate();

  const { isPending, mutateAsync: execute } = useSubmitTx({
    toast: {
      success: () => "Message executed successfully",
      error: (error) => `Error executing message: ${error.message}`,
    },
    mutation: {
      mutationFn: async () => {
        if (!signingClient || !account)
          throw new Error("Signing client or account address is not available");
        await signingClient?.execute({ sender: account.address, execute: JSON.parse(msg) });
      },
    },
  });

  useEffect(() => {
    if (!account) navigate({ to: "/" });
  }, [account]);

  useEffect(() => {
    const resizeMonaco = () => editorRef.current.layout({ width: 100, height: 100 });
    window.addEventListener("resize", resizeMonaco);
    return () => window.removeEventListener("resize", resizeMonaco);
  }, []);

  const handlerOnMount: OnMount = (editor, monaco) => {
    editorRef.current = editor;
  };

  return (
    <div className="w-full md:max-w-[50rem] mx-auto flex flex-col p-4 pt-6 gap-4 min-h-[100svh] md:min-h-[80svh]">
      <Editor
        onMount={handlerOnMount}
        language="json"
        theme={theme === "dark" ? "vs-dark" : "vs-light"}
        width="100%"
        height="60vh"
        value={msg}
        onChange={(v) => setMsg(v ?? "")}
        options={{
          scrollbar: {
            verticalScrollbarSize: 0,
          },
          minimap: {
            enabled: false,
          },
        }}
      />

      <div className="absolute right-10 bottom-10 flex justify-end">
        <Button disabled={isPending} onClick={() => execute()}>
          Execute Message
        </Button>
      </div>
    </div>
  );
}
