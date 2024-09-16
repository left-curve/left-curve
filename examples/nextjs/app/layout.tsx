import type { Metadata } from "next";
import { Inter, Space_Grotesk } from "next/font/google";
import { Providers } from "./providers";

import { ExampleHeader } from "@leftcurve/react/components/examples";

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
  title: "SuperApp",
  description: "A super app",
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
          <main className="flex flex-1 bg-stone-200">{children}</main>
        </Providers>
      </body>
    </html>
  );
}
