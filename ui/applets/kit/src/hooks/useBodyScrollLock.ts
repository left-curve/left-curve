import { useEffect } from "react";

type Saved = {
  y: number;
  body: {
    position: string;
    top: string;
    left: string;
    right: string;
    width: string;
    paddingRight: string;
  };
} | null;

let lockCount = 0;
let saved: Saved = null;

function applyLock() {
  const body = document.body;

  saved = {
    y: window.scrollY,
    body: {
      position: body.style.position,
      top: body.style.top,
      left: body.style.left,
      right: body.style.right,
      width: body.style.width,
      paddingRight: body.style.paddingRight,
    },
  };

  body.dataset.scrollLock = "1";
  body.dataset.scrollLockY = String(saved.y);

  body.style.position = "fixed";
  body.style.top = `-${saved.y}px`;
  body.style.left = "0";
  body.style.right = "0";
  body.style.width = "100%";
}

function releaseLock() {
  if (!saved) return;
  const body = document.body;

  body.style.position = saved.body.position;
  body.style.top = saved.body.top;
  body.style.left = saved.body.left;
  body.style.right = saved.body.right;
  body.style.width = saved.body.width;
  body.style.paddingRight = saved.body.paddingRight;

  delete body.dataset.scrollLock;
  delete body.dataset.scrollLockY;

  window.scrollTo(0, saved.y);
  saved = null;
}

export function useBodyScrollLock(locked: boolean) {
  useEffect(() => {
    if (typeof window === "undefined") return;

    if (locked) {
      if (lockCount++ === 0) applyLock();
    } else {
      if (lockCount > 0 && --lockCount === 0) releaseLock();
    }
    return () => {
      if (locked) {
        if (lockCount > 0 && --lockCount === 0) releaseLock();
      }
    };
  }, [locked]);
}
