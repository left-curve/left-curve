export function cloneCardForExport(clone: HTMLElement): void {
  // Force desktop sizes for the character image (overrides responsive Tailwind classes)
  const characterImg = clone.querySelector("img[alt='character']") as HTMLElement | null;
  if (characterImg) {
    characterImg.style.height = "100%";
    characterImg.style.maxHeight = "24rem";
  }
}
