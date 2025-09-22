import * as webllm from "@mlc-ai/web-llm";
import { StreamParser } from "./parser";

export type Tool = { name: string; fn: <P, R>(params: P) => Promise<R> };

export type LLMessage =
  | { type: "thinking"; content: string; author: "assistant" }
  | { type: "function"; content: string; author: "assistant" }
  | { type: "text"; content: string; author: "user" | "assistant" };

export function systemPrompt(tools: Array<webllm.ChatCompletionTool>) {
  return `
  # Tool Instructions
    - Only when looking for real time information use relevant functions if available, otherwise answer based on your knowledge.
    - Not all questions require a function call.
    You have access to the following functions:

    ${JSON.stringify(tools, null, 2)}

    If a you choose to call a function ONLY reply in the following format:
    <function>{"name": function name, "parameters": dictionary of argument name and its value}</function>

    Reminder:
    - Function calls MUST follow the specified format and use BOTH <function> and </function>
    - Required parameters MUST be specified
    - Only call one function at a time
    - When calling a function, do NOT add any other words, ONLY the function calling
    - Put the entire function call reply on one line
    - Always add your sources when using search results to answer the user query

    **If no tool is needed to answer the user, you must respond directly with a helpful, plain text answer. Do not use any tags in this case.**

    You are a helpful Assistant.
  `;
}

export async function loadModel(_modelId: string): Promise<webllm.MLCEngine> {
  const engine = new webllm.MLCEngine({
    initProgressCallback: (p) => console.log(p),
  });
  engine.setAppConfig({
    model_list: [
      {
        model: "https://huggingface.co/mlc-ai/Qwen3-1.7B-q4f16_1-MLC",
        model_id: "Qwen3-1.7B-q4f16_1-MLC",
        model_lib:
          "https://raw.githubusercontent.com/mlc-ai/binary-mlc-llm-libs/main/web-llm-models/" +
          "v0_2_48" +
          "/Qwen3-1.7B-q4f16_1-ctx4k_cs1k-webgpu.wasm",
        vram_required_MB: 2036.66,
        low_resource_required: true,
        overrides: {
          context_window_size: 4096,
        },
      },
    ],
  });

  try {
    await engine.reload("Qwen3-1.7B-q4f16_1-MLC");
    return engine;
  } catch (error) {
    console.error("Error loading model:", error);
    throw error;
  }
}

export async function handleStreamResponse(
  engine: webllm.MLCEngine,
  tools: Tool[],
  request: webllm.ChatCompletionRequestStreaming,
  updateMessages: (messages: LLMessage[]) => void,
  messages: LLMessage[] = [],
) {
  const parser = new StreamParser();
  let finalMessages: LLMessage[] = [];

  const asyncChunkGenerator = await engine.chat.completions.create(request);
  for await (const chunk of asyncChunkGenerator) {
    const delta = chunk.choices[0]?.delta?.content || "";
    finalMessages = parser.feed(delta);

    if (finalMessages.length > 1 && finalMessages[0].type === "thinking") {
      finalMessages.shift();
    }

    updateMessages([...messages, ...finalMessages]);
  }

  const last = finalMessages[finalMessages.length - 1];

  if (last?.type === "function") {
    try {
      const { name, parameters } = JSON.parse(last.content);
      const { fn } = tools.find((t) => t.name === name) || {};
      if (!fn) return;
      const result = await fn(parameters);

      const followUpRequest: webllm.ChatCompletionRequestStreaming = {
        stream: true,
        messages: [
          ...request.messages,
          {
            role: "assistant",
            content: `<function>${last.content}</function>`,
          },
          { role: "user", content: JSON.stringify(result) },
        ],
      };

      await handleStreamResponse(engine, tools, followUpRequest, updateMessages, [
        ...messages,
        ...finalMessages,
      ]);
    } catch (err) {
      console.error("Error parsing function call:", err);
    }
  }
}
