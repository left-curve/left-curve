import { useId, useState } from "react";

export type InstallTabsCommand = {
  label: string;
  command: string;
};

export type InstallTabsProps = {
  commands: readonly InstallTabsCommand[];
  defaultIndex?: number;
};

/**
 * Tabbed install widget. Visually mirrors Vocs' `HomePage.InstallPackage`
 * but accepts arbitrary `{ label, command }` pairs so it works for Python
 * (uv / pip / poetry), Rust (cargo add / Cargo.toml), and anything else
 * outside the npm/pnpm/yarn shape.
 */
export function InstallTabs({ commands, defaultIndex = 0 }: InstallTabsProps) {
  const [active, setActive] = useState(defaultIndex);
  const groupId = useId();
  const current = commands[active];
  const isMultiLine = current.command.includes("\n");

  return (
    <div className="docs-InstallTabs">
      <div role="tablist" className="docs-InstallTabs__list">
        {commands.map((c, i) => {
          const isActive = i === active;
          return (
            <button
              key={`${groupId}-${i}`}
              id={`${groupId}-tab-${i}`}
              role="tab"
              type="button"
              aria-selected={isActive}
              tabIndex={isActive ? 0 : -1}
              data-active={isActive}
              className="docs-InstallTabs__trigger"
              onClick={() => setActive(i)}
              onKeyDown={(e) => {
                if (e.key === "ArrowRight") setActive((active + 1) % commands.length);
                if (e.key === "ArrowLeft")
                  setActive((active - 1 + commands.length) % commands.length);
              }}
            >
              {c.label}
            </button>
          );
        })}
      </div>
      <div className="docs-InstallTabs__panel" data-multiline={isMultiLine}>
        <pre className="docs-InstallTabs__code">
          <code>{current.command}</code>
        </pre>
        <button
          type="button"
          className="docs-InstallTabs__copy"
          aria-label="Copy"
          onClick={() => {
            void navigator.clipboard.writeText(current.command);
          }}
        >
          Copy
        </button>
      </div>
      <style>{`
        .docs-InstallTabs {
          min-width: 0;
          border: 1px solid var(--vocs-color_border, rgba(127, 127, 127, 0.2));
          border-radius: 10px;
          background: var(--vocs-color_codeBlockBackground, var(--vocs-color_background2));
          overflow: hidden;
        }
        .docs-InstallTabs__list {
          display: flex;
          gap: 0;
          border-bottom: 1px solid var(--vocs-color_border, rgba(127, 127, 127, 0.2));
          background: var(--vocs-color_background2, transparent);
        }
        .docs-InstallTabs__trigger {
          appearance: none; background: transparent; border: none;
          color: var(--vocs-color_text3);
          cursor: pointer; font: inherit;
          font-size: 0.8125rem;
          padding: 0.5rem 0.875rem;
          border-bottom: 2px solid transparent;
          transform: translateY(1px);
        }
        .docs-InstallTabs__trigger:hover {
          color: var(--vocs-color_text2);
        }
        .docs-InstallTabs__trigger[data-active="true"] {
          color: var(--vocs-color_text);
          border-bottom-color: var(--vocs-color_textAccent, var(--vocs-color_borderAccent));
        }
        .docs-InstallTabs__panel {
          position: relative;
          padding: 0.75rem 1rem;
          font-family: var(--vocs-fontFamily_mono, ui-monospace, monospace);
          font-size: 0.875rem;
          line-height: 1.5;
        }
        .docs-InstallTabs__code {
          margin: 0;
          padding: 0;
          background: transparent;
          color: var(--vocs-color_text);
          overflow-x: auto;
          white-space: pre;
        }
        .docs-InstallTabs__code code {
          background: transparent;
          padding: 0;
          font-family: inherit;
          font-size: inherit;
          color: inherit;
        }
        .docs-InstallTabs__copy {
          position: absolute;
          top: 0.5rem; right: 0.5rem;
          appearance: none;
          background: var(--vocs-color_background2);
          border: 1px solid var(--vocs-color_border);
          border-radius: 6px;
          color: var(--vocs-color_text3);
          cursor: pointer;
          font: inherit;
          font-size: 0.6875rem;
          padding: 0.125rem 0.5rem;
          opacity: 0;
          transition: opacity 0.15s ease;
        }
        .docs-InstallTabs__panel:hover .docs-InstallTabs__copy,
        .docs-InstallTabs__copy:focus-visible {
          opacity: 1;
        }
        .docs-InstallTabs__copy:hover {
          color: var(--vocs-color_text);
        }
      `}</style>
    </div>
  );
}
