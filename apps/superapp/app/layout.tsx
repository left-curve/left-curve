import type { Metadata } from "next";
import { Inter, Space_Grotesk } from "next/font/google";
import { Providers } from "./providers";

import { Header } from "@leftcurve/react/components";

import "@leftcurve/react/fonts/ABCDiatypeRounded/index.css"
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
          <Header />
          <main className="flex flex-1 bg-stone-50">{children}</main>
        </Providers>
      </body>
    </html>
  );
}
