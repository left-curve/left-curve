import type { ReactNode } from "react";

/**
 * <PackageCard> — a normalized landing-page card for one SDK.
 *
 * The current landing uses three `<div className="sdk-card">` blocks with
 * different inner content. Two cards (Python, Rust) carry a one-line code
 * fence; one card (TypeScript) carries `HomePage.InstallPackage`, which
 * renders a 3-tab control. The tab control pushes the TS card visibly taller
 * than its peers, breaking the grid rhythm.
 *
 * This component forces:
 *  - identical header/body/footer regions (CSS grid rows: auto 1fr auto)
 *  - identical install-line height (`min-height` on the install slot)
 *  - one footer link, right-aligned, with a consistent affordance
 *
 * It does NOT replace `HomePage.InstallPackage` — that component is fine on
 * its own. The card just gives every card the same vertical budget.
 *
 * MDX usage:
 *
 *   import { PackageCard, PackageCardGrid } from '../components/PackageCard'
 *   import { HomePage } from 'vocs/components'
 *
 *   <PackageCardGrid>
 *     <PackageCard
 *       title="TypeScript"
 *       packageName="@left-curve/sdk"
 *       summary="viem-style client. Public and signer clients, action functions, WebSocket subscriptions, tree-shakable imports."
 *       bestFor="browser dApps, Node.js services, indexers, automated traders."
 *       install={<HomePage.InstallPackage name="@left-curve/sdk" />}
 *       href="/typescript/getting-started/installation"
 *     />
 *     ...
 *   </PackageCardGrid>
 */

export type PackageCardProps = {
  title: string;
  packageName: string;
  summary: string;
  bestFor: string;
  href: string;
  /**
   * Inline JSX install snippet (e.g., `<HomePage.InstallPackage />` or
   * `<InstallTabs />`). Mutually exclusive with `children`.
   */
  install?: ReactNode;
  /**
   * MDX content for the install slot. Use this when you want a markdown
   * code fence (so Vocs/Shiki syntax-highlights the snippet).
   */
  children?: ReactNode;
};

export function PackageCard({
  title,
  packageName,
  summary,
  bestFor,
  install,
  children,
  href,
}: PackageCardProps) {
  return (
    <article className="docs-PackageCard">
      <h3 className="docs-PackageCard__title">{title}</h3>
      <p className="docs-PackageCard__summary">
        <code>{packageName}</code> — {summary}
      </p>
      <p className="docs-PackageCard__bestFor">
        <strong>Best for:</strong> {bestFor}
      </p>
      <div className="docs-PackageCard__install">{children ?? install}</div>
      <p className="docs-PackageCard__link">
        <a href={href}>Get started →</a>
      </p>
    </article>
  );
}

export type PackageCardGridProps = { children: ReactNode };

export function PackageCardGrid({ children }: PackageCardGridProps) {
  return (
    <div className="docs-PackageCardGrid">
      {children}
      <style>{`
        .docs-PackageCardGrid {
          display: grid;
          grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
          gap: 1.25rem;
          margin: 2.5rem 0 1.5rem;
        }
        .docs-PackageCard {
          display: grid;
          grid-template-rows: auto auto auto 1fr auto;
          gap: 0.75rem;
          padding: 1.25rem 1.5rem;
          border: 1px solid var(--vocs-color_border, rgba(127, 127, 127, 0.2));
          border-radius: 12px;
          background: var(--vocs-color_background2, transparent);
          transition: border-color 0.2s ease, transform 0.2s ease;
        }
        .docs-PackageCard:hover {
          border-color: var(--vocs-color_borderAccent, rgba(127, 127, 127, 0.5));
          transform: translateY(-2px);
        }
        .docs-PackageCard__title {
          margin: 0 !important;
          font-size: 1.125rem;
        }
        .docs-PackageCard__summary,
        .docs-PackageCard__bestFor {
          margin: 0 !important;
          font-size: 0.9375rem;
          line-height: 1.45;
        }
        /* The install slot reserves the height of a 3-tab install widget so
           cards with a plain code fence don't read as "shorter". */
        .docs-PackageCard__install {
          min-height: 6.5rem;
          display: flex;
          align-items: flex-start;
        }
        .docs-PackageCard__install > * {
          width: 100%;
          margin: 0 !important;
        }
        .docs-PackageCard__link {
          margin: 0 !important;
          font-size: 0.9375rem;
        }
        .docs-PackageCard__link a {
          font-weight: 500;
        }
      `}</style>
    </div>
  );
}
