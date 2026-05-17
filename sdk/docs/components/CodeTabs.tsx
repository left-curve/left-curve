import {
  Children,
  type ReactElement,
  type ReactNode,
  isValidElement,
  useId,
  useState,
} from "react";

/**
 * <CodeTabs> — a minimal accessible tabs widget for two-to-N code variants.
 *
 * Vocs does not publicly export its internal `Tabs` component, so this is a
 * direct, hand-rolled replacement. Uses only Vocs CSS variables so it inherits
 * theming for free (light/dark).
 *
 * MDX usage:
 *
 *   import { CodeTabs, CodeTab } from '../../components/CodeTabs'
 *
 *   <CodeTabs labels={["Extended", "Tree-shakable"]}>
 *     <CodeTab>
 *       ```ts
 *       const amount = await client.getBalance({ address, denom: "dango" })
 *       ```
 *     </CodeTab>
 *     <CodeTab>
 *       ```ts
 *       const amount = await getBalance(client, { address, denom: "dango" })
 *       ```
 *     </CodeTab>
 *   </CodeTabs>
 *
 * Notes:
 *  - Labels are an explicit prop, not children, so MDX whitespace doesn't
 *    drift into the tab triggers. Order matches `<CodeTab>` child order.
 *  - `defaultIndex` lets a page pin a non-zero starting tab (e.g., pnpm).
 *  - No external state library: a single `useState` is enough.
 */
export type CodeTabsProps = {
  labels: readonly string[];
  defaultIndex?: number;
  children: ReactNode;
};

export function CodeTabs({ labels, defaultIndex = 0, children }: CodeTabsProps) {
  const [active, setActive] = useState(defaultIndex);
  const groupId = useId();
  const panels = Children.toArray(children).filter(isCodeTab);

  if (panels.length !== labels.length) {
    throw new Error(
      `<CodeTabs>: labels.length (${labels.length}) must match <CodeTab> children count (${panels.length}).`,
    );
  }

  return (
    <div className="docs-CodeTabs">
      <div role="tablist" aria-orientation="horizontal" className="docs-CodeTabs__list">
        {labels.map((label, i) => {
          const isActive = i === active;
          const tabId = `${groupId}-tab-${i}`;
          const panelId = `${groupId}-panel-${i}`;
          return (
            <button
              key={tabId}
              id={tabId}
              role="tab"
              type="button"
              aria-selected={isActive}
              aria-controls={panelId}
              tabIndex={isActive ? 0 : -1}
              className="docs-CodeTabs__trigger"
              data-active={isActive}
              onClick={() => setActive(i)}
              onKeyDown={(e) => {
                if (e.key === "ArrowRight") setActive((active + 1) % labels.length);
                if (e.key === "ArrowLeft")
                  setActive((active - 1 + labels.length) % labels.length);
                if (e.key === "Home") setActive(0);
                if (e.key === "End") setActive(labels.length - 1);
              }}
            >
              {label}
            </button>
          );
        })}
      </div>
      {panels.map((panel, i) => {
        const isActive = i === active;
        const tabId = `${groupId}-tab-${i}`;
        const panelId = `${groupId}-panel-${i}`;
        return (
          <div
            key={panelId}
            id={panelId}
            role="tabpanel"
            aria-labelledby={tabId}
            hidden={!isActive}
            className="docs-CodeTabs__panel"
          >
            {panel}
          </div>
        );
      })}
      <style>{`
        .docs-CodeTabs {
          margin: 1.5rem 0;
        }
        .docs-CodeTabs__list {
          display: flex;
          gap: 0.25rem;
          border-bottom: 1px solid var(--vocs-color_border);
          margin-bottom: 0;
        }
        .docs-CodeTabs__trigger {
          appearance: none;
          background: transparent;
          border: none;
          border-bottom: 2px solid transparent;
          color: var(--vocs-color_text3);
          cursor: pointer;
          font: inherit;
          font-size: 0.8125rem;
          padding: 0.5rem 0.75rem;
          transform: translateY(1px);
        }
        .docs-CodeTabs__trigger:hover {
          color: var(--vocs-color_text2);
        }
        .docs-CodeTabs__trigger[data-active="true"] {
          color: var(--vocs-color_text);
          border-bottom-color: var(--vocs-color_textAccent, var(--vocs-color_borderAccent));
        }
        .docs-CodeTabs__panel {
          padding-top: 0.25rem;
        }
        /* Collapse the default top margin Vocs gives the first <pre> inside the panel. */
        .docs-CodeTabs__panel > :first-child { margin-top: 0.5rem !important; }
        .docs-CodeTabs__panel > :last-child { margin-bottom: 0 !important; }
      `}</style>
    </div>
  );
}

export type CodeTabProps = { children: ReactNode };

export function CodeTab({ children }: CodeTabProps) {
  return <>{children}</>;
}
CodeTab.displayName = "CodeTab";

function isCodeTab(node: ReactNode): node is ReactElement<CodeTabProps> {
  return (
    isValidElement(node) &&
    typeof node.type !== "string" &&
    (node.type as { displayName?: string }).displayName === "CodeTab"
  );
}
