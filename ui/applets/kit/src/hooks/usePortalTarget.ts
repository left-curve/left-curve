import { useEffect, useState } from "react";

export function usePortalTarget(querySelector: string): HTMLElement | null {
  const [mountNode, setMountNode] = useState<HTMLElement | null>(null);

  useEffect(() => {
    const targetElement = document.querySelector(querySelector) as HTMLElement;

    if (targetElement) setMountNode(targetElement);
  }, [querySelector]);

  return mountNode;
}
