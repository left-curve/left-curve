import type { Metadata } from "next";
import { Providers } from "./providers";

import { Header } from "~/components/Header";

import "@leftcurve/dango/fonts/ABCDiatypeRounded/index.css";
import "~/public/styles/globals.css";

export const metadata: Metadata = {
  title: "Dango Portal App",
  description: "",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className="flex flex-col min-h-screen w-full h-full bg-white relative overflow-x-auto overflow-y-hidden scrollbar-none items-center justify-center pt-[166px] md:pt-[110px]">
        <img
          src="/images/background.png"
          alt="bg-image"
          className="object-cover h-[80vh] absolute top-[15%] left-1/2 transform -translate-x-1/2 z-0 blur-2xl "
        />
        <Providers>
          <Header />
          <main className="flex flex-1 w-full">{children}</main>
        </Providers>
      </body>
    </html>
  );
}
