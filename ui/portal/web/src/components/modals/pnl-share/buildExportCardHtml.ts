export function cloneCardForExport(clone: HTMLElement): void {
  // Force desktop sizes for the character image (overrides responsive Tailwind classes)
  const characterImg = clone.querySelector("img[alt='character']") as HTMLElement | null;
  if (characterImg) {
    characterImg.style.height = "100%";
    characterImg.style.maxHeight = "24rem";
  }

  // Force desktop layout for the prices row (otherwise stays stacked on mobile viewport)
  const pricesRow = clone.querySelector('[data-pnl="prices-row"]') as HTMLElement | null;
  if (pricesRow) {
    pricesRow.style.flexDirection = "row";
    pricesRow.style.gap = "1.5rem";
  }
}
