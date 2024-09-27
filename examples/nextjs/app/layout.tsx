import type { Metadata } from "next";
import { Inter, Space_Grotesk } from "next/font/google";
import { Providers } from "./providers";

import { ExampleHeader } from "@leftcurve/dango/components/examples";

import "@leftcurve/dango/fonts/ABCDiatypeRounded/index.css";
import "../public/styles/globals.css";

const inter = Inter({
  variable: "--font-inter",
  display: "optional",
  subsets: ["latin"],
});

const grotesk = Space_Grotesk({
  variable: "--font-grotesk",
  display: "optional",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "Nextjs Example",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className={`${inter.variable} ${grotesk.variable} flex flex-col min-h-screen w-full`}>
        <Providers>
          <ExampleHeader />
          <main className="flex flex-1 bg-stone-50">{children}</main>
        </Providers>
      </body>
    </html>
  );
}
