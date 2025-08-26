import { lazy } from "react";

import type React from "react";
import type { JsonString, JsonValue, OneRequired } from "@left-curve/dango/types";

type JsonVisualizerProps = {
  collapsed?: boolean | number;
} & OneRequired<{ json: JsonValue; string: JsonString }, "json", "string">;

const JsonView = lazy(() => import("@microlink/react-json-view"));

export const JsonVisualizer: React.FC<JsonVisualizerProps> = ({ json, string, collapsed }) => {
  const source = string ? JSON.parse(string) : json;

  return (
    <JsonView
      name={false}
      src={source}
      displayObjectSize={false}
      displayDataTypes={false}
      collapsed={collapsed}
      indentWidth={1}
      theme={{
        base00: "#453d39",
        base01: "#fffcf6",
        base02: "#453d39",
        base03: "#fffcf6",
        base04: "#fffcf6",
        base05: "#fffcf6",
        base06: "#fffcf6",
        base07: "#fff3e1", // Keys
        base08: "#fffcf6",
        base09: "#fffcf6", // Values
        base0A: "#fffcf6",
        base0B: "#fffcf6",
        base0C: "#fffcf6",
        base0D: "#d4882c", // Arrow Open
        base0E: "#d4882c", // Arrow Close
        base0F: "#fffcf6",
      }}
    />
  );
};
