import type * as webllm from "@mlc-ai/web-llm";
import { useCallback, useEffect, useRef, useState } from "react";

import { handleStreamResponse, loadModel, systemPrompt } from "./model";
import type { LLMessage } from "./model";

type UseChatModelParameters = {
  engine: webllm.MLCEngine;
  toolsDefinition: webllm.ChatCompletionTool[];
  toolsImplementation: { name: string; fn: <P, R>(params: P) => Promise<R> }[];
};

export function useChatModel(parameters: UseChatModelParameters) {
  const { toolsDefinition, toolsImplementation, engine } = parameters;
  const modelRef = useRef<webllm.MLCEngine | null>(null);
  const [messages, setMessages] = useState<LLMessage[]>([]);

  const [isLoading, setIsLoading] = useState<boolean>(false);

  useEffect(() => {
    setIsLoading(true);
    loadModel(engine).then((engine) => {
      modelRef.current = engine;
      setIsLoading(false);
    });
  }, []);

  const sendMessage = useCallback(
    async (input: string) => {
      if (!modelRef.current) return;
      setIsLoading(true);

      const userMessage: LLMessage = {
        type: "text",
        content: input,
        author: "user",
      };
      const newMessages = [...messages, userMessage];
      setMessages(newMessages);

      const history = newMessages
        .filter((m) => m.type !== "thinking")
        .map((m) => {
          if (m.type === "function") {
            return {
              role: "assistant",
              content: `<function>${m.content}</function>`,
            };
          }
          return { role: m.author, content: m.content };
        });

      const request: webllm.ChatCompletionRequestStreaming = {
        stream: true,
        messages: [
          { role: "system", content: systemPrompt(toolsDefinition) },
          ...(history as webllm.ChatCompletionMessageParam[]),
        ],
      };

      try {
        await handleStreamResponse(
          modelRef.current,
          toolsImplementation,
          request,
          setMessages,
          newMessages,
        );
      } finally {
        setIsLoading(false);
      }
    },
    [messages],
  );
  return { sendMessage, messages, isLoading };
}
