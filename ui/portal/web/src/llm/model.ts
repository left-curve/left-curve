import type * as webllm from "@mlc-ai/web-llm";
import { StreamParser } from "./parser";

export type Tool = { name: string; fn: <P, R>(params: P) => Promise<R> };

export type LLMessage =
  | { type: "thinking"; content: string; author: "assistant" }
  | { type: "function"; content: string; author: "assistant" }
  | { type: "text"; content: string; author: "user" | "assistant" };

export function systemPrompt(tools: Array<webllm.ChatCompletionTool>) {
  return `
  # Tool Instructions
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
    - You can NOT call functions that are not listed in the tool list

    **After a function call transform the response in natural language to the user.**

    If you do not need to call a function, respond in natural language.

    You are a helpful Assistant.
  `;
}

export async function loadModel(engine: webllm.MLCEngine): Promise<webllm.MLCEngine> {
  const appConfig = {
    model_list: [
      {
        model: "https://huggingface.co/mlc-ai/Hermes-2-Pro-Llama-3-8B-q4f16_1-MLC",
        model_id: "Hermes-2-Pro-Llama-3-8B-q4f16_1-MLC",
        model_lib:
          "https://raw.githubusercontent.com/mlc-ai/binary-mlc-llm-libs/main/web-llm-models/" +
          "v0_2_48" +
          "/Llama-3-8B-Instruct-q4f16_1-ctx4k_cs1k-webgpu.wasm",
        vram_required_MB: 4976.13,
        low_resource_required: false,
        overrides: {
          context_window_size: 4096,
        },
      },
      {
        model: "https://huggingface.co/mlc-ai/Qwen3-4B-q4f32_1-MLC",
        model_id: "Qwen3-4B-q4f32_1-MLC",
        model_lib:
          "https://raw.githubusercontent.com/mlc-ai/binary-mlc-llm-libs/main/web-llm-models/" +
          "v0_2_80" +
          "/Qwen3-4B-q4f32_1-ctx4k_cs1k-webgpu.wasm",
        vram_required_MB: 4327.71,
        low_resource_required: true,
        overrides: {
          context_window_size: 4096,
        },
      },
      {
        model: "https://huggingface.co/mlc-ai/Qwen3-8B-q4f32_1-MLC",
        model_id: "Qwen3-8B-q4f32_1-MLC",
        model_lib:
          "https://raw.githubusercontent.com/mlc-ai/binary-mlc-llm-libs/main/web-llm-models/" +
          "v0_2_80" +
          "/Qwen3-8B-q4f32_1-ctx4k_cs1k-webgpu.wasm",
        vram_required_MB: 5695.78,
        low_resource_required: false,
        overrides: {
          context_window_size: 4096,
        },
      },
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
      {
        model: "https://huggingface.co/mlc-ai/gemma-3-1b-it-q4f16_1-MLC",
        model_id: "gemma-3-1b-it-q4f16_1-MLC",
        model_lib:
          "https://raw.githubusercontent.com/mlc-ai/binary-mlc-llm-libs/main/web-llm-models/" +
          "v0_2_80" +
          "/gemma3-1b-it-q4f16_1-ctx4k_cs1k-webgpu.wasm",
      },
    ],
  };

  try {
    // engine.setAppConfig(appConfig);
    await engine.reload("Qwen3-4B-q4f32_1-MLC");
    // await engine.reload("Qwen3-8B-q4f32_1-MLC");
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

    /*     if (finalMessages.length > 1 && finalMessages[0].type === "thinking") {
      finalMessages.shift();
    } */
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
          {
            role: "user",
            content: `<tool_response>\n${JSON.stringify(result)}\n</tool_response>`,
          },
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
