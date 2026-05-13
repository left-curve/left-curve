import { toPng } from "html-to-image";

const waitForImages = (root: HTMLElement) => {
  const images = root.querySelectorAll("img");
  return Promise.all(
    Array.from(images).map(
      (img) =>
        new Promise<void>((resolve) => {
          if (img.complete) return resolve();
          img.onload = () => resolve();
          img.onerror = () => resolve();
        }),
    ),
  );
};

type SaveCardAsImageOptions = {
  source: HTMLElement;
  prepareClone?: (clone: HTMLElement) => void;
  filename: string;
  width?: number;
};

export async function saveCardAsImage({
  source,
  prepareClone,
  filename,
  width = 500,
}: SaveCardAsImageOptions): Promise<void> {
  const clone = source.cloneNode(true) as HTMLElement;
  clone.dataset.export = "true";
  prepareClone?.(clone);
  clone.style.width = `${width}px`;

  const container = document.createElement("div");
  container.style.cssText = "position:fixed;left:-9999px;top:0;";
  container.appendChild(clone);
  document.body.appendChild(container);

  try {
    await waitForImages(clone);
    const dataUrl = await toPng(clone, { cacheBust: true });
    const link = document.createElement("a");
    link.download = filename;
    link.href = dataUrl;
    link.click();
  } finally {
    document.body.removeChild(container);
  }
}
