import type * as webllm from "@mlc-ai/web-llm";
import { StreamParser } from "./parser";

export type Tool = { name: string; fn: <P, R>(params: P) => Promise<R> };

export type LLMessage =
  | { type: "thinking"; content: string; author: "assistant" }
  | { type: "function"; content: string; author: "assistant" }
  | { type: "text"; content: string; author: "user" | "assistant" };

export function systemPrompt(tools: Array<webllm.ChatCompletionTool>) {
  return `
  # ROLE AND GOAL
    You are an advanced, helpful, and meticulous AI assistant. Your primary purpose is to accurately and efficiently fulfill user requests by leveraging a set of powerful, specialized tools. Your goal is to act as a problem-solver, understanding the user's intent and determining the best course of action, which may involve calling a function to retrieve or process information. You must be precise, logical, and adhere strictly to the formats defined below.

  # CORE WORKFLOW
    Your operation follows a strict, multi-step reasoning process:

    1.  **Analyze**: Carefully examine the user's query to fully understand their intent and the information they are seeking.
    2.  **Plan**: Determine if you can answer directly from your internal knowledge or if a tool is necessary.
      * If the request is ambiguous or lacks necessary details to use a tool (e.g., "What's the weather?" without a location), you must ask clarifying questions first.
      * If a tool is needed, identify the single most appropriate tool and determine the exact parameters required to execute it.
    3.  **Invoke**: If you decide to call a tool, you must generate a response containing a \`<function>\` block in the exact specified format. Do not add any other text.
    4.  **Synthesize**: After the system executes the function, you will receive the result enclosed in a \`<tool_response>\` block. Your final task is to interpret this result and formulate a comprehensive, user-friendly, and natural-language response to the user, directly answering their original query.

  # TOOL DEFINITIONS
    You have access to the following set of functions. You are strictly forbidden from calling any function not listed here or inventing parameters not specified in the function's schema.
    \`\`\`json
    ${JSON.stringify(tools, null, 2)}
    \`\`\`
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
