import type { Metadata } from "next";
import "./globals.css";

import "@dango/assets/fonts/ABCDiatypeRounded/index.css";
import "@dango/assets/fonts/Exposure/index.css";

export const metadata: Metadata = {
  title: "Dango",
  description: "Bringing back the good things of the last cycle",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className="flex flex-col min-h-screen w-full h-full bg-white relative scrollbar-none items-center justify-center overflow-y-hidden">
        <div className="fixed mx-0 top-6 z-50">
          <img src="/images/dango.svg" alt="logo" className="h-6 md:h-12 object-contain" />
        </div>
        {children}
      </body>
    </html>
  );
}
