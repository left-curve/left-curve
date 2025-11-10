import { lazy, useState } from "react";

import { twMerge } from "@left-curve/foundation";

import { IconChevronDownFill } from "./icons/IconChevronDownFill";

import type React from "react";
import type { JsonString, JsonValue, OneRequired } from "@left-curve/dango/types";

type JsonVisualizerProps = {
  collapsed?: boolean | number;
} & OneRequired<{ json: JsonValue; string: JsonString }, "json", "string">;

const JsonView = lazy(() => import("@microlink/react-json-view"));

export const JsonVisualizer: React.FC<JsonVisualizerProps> = ({ json, string, collapsed }) => {
  const [isCollapsed, setCollapsed] = useState(collapsed);
  const source = string ? JSON.parse(string) : json;

  return (
    <div className="relative">
      <JsonView
        name={false}
        src={source}
        shouldCollapse={() => false}
        displayObjectSize={false}
        displayDataTypes={false}
        collapsed={isCollapsed}
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
      <div className="absolute top-0 right-0">
        <button
          type="button"
          className="text-primitives-white-light-100 outline-none focus:outline-none"
          onClick={() => setCollapsed(isCollapsed === collapsed ? false : collapsed)}
        >
          <IconChevronDownFill
            className={twMerge(
              "w-4 h-4 transition-all",
              isCollapsed || typeof isCollapsed === "number" ? "rotate-0" : "rotate-180",
            )}
          />
        </button>
      </div>
    </div>
  );
};
