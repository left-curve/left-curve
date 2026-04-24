import type React from "react";

export const PageGlow: React.FC = () => (
  <div className="fixed top-0 left-0 w-screen h-screen bg-[radial-gradient(ellipse_at_center_40%,rgba(245,221,184,0.6)_0%,rgba(245,221,184,0.2)_30%,transparent_60%)] pointer-events-none -z-10" />
);
