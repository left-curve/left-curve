import { useAccount, useConfig, usePublicClient, useSigningClient } from "@left-curve/store";
import { useMemo } from "react";
import { createDangoTools } from "~/llm/tools";
import { useChatModel } from "~/llm/useChatModel";
import { MessageRenderer } from "./MessageRenderer";
import { ChatInput } from "./ChatInput";

import type React from "react";
import { MLCEngine } from "@mlc-ai/web-llm";

const engine = new MLCEngine({ initProgressCallback: (p) => console.log(p) });

export const ChatBot: React.FC = () => {
  const publicClient = usePublicClient();
  const { data: signingClient } = useSigningClient();
  const { coins } = useConfig();
  const { account } = useAccount();

  const { toolsDefinition, toolsImplementation } = useMemo(() => {
    const tools = createDangoTools(
      engine,
      signingClient ? signingClient : publicClient,
      account ? account : null,
      coins.byDenom,
    );
    const definitions = tools.map((tool) => tool.definition);
    const implementations = tools.map((tool) => ({
      name: tool.definition.function.name,
      fn: tool.fn,
    }));
    return { toolsDefinition: definitions, toolsImplementation: implementations };
  }, [publicClient, signingClient]);

  const { sendMessage, messages, isLoading } = useChatModel({
    engine,
    toolsDefinition,
    toolsImplementation,
  });

  return (
    <div className="w-full md:max-w-3xl mx-auto flex flex-col p-4 pt-6 gap-4 min-h-[100svh] md:min-h-fit">
      <div className="flex-grow overflow-y-auto pr-4">
        <div>
          {messages.map((m, index) => (
            <MessageRenderer key={index as React.Key} message={m} />
          ))}
        </div>
      </div>

      <ChatInput isLoading={isLoading} handler={sendMessage} />
    </div>
  );
};
