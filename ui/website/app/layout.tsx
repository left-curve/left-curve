import type { Metadata } from "next";
import "./globals.css";

import "@left-curve/ui-config/fonts/ABCDiatypeRounded/index.css";
import "@left-curve/ui-config/fonts/Exposure/index.css";

export const metadata: Metadata = {
  title: "Dango",
  description: "A DeFi hub with novel leverage capabilities and a true next-gen user experience",
  icons: {
    icon: "/favicon.svg",
  },
  openGraph: {
    images: [
      {
        url: "/images/og.png",
        width: 1200,
        height: 630,
      },
    ],
  },
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
          <img src="/images/dango.svg" alt="logo" className="h-10 md:h-16 object-contain" />
        </div>
        {children}
      </body>
    </html>
  );
}
