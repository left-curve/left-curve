import type { Metadata } from "next";
import { Providers } from "./providers";

import "@dango/shared/fonts/ABCDiatypeRounded/index.css";
import "~/public/styles/globals.css";

export const metadata: Metadata = {
  title: "Dango App",
  description: "",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className="flex flex-col min-h-screen w-full h-full bg-white relative overflow-y-auto overflow-x-hidden scrollbar-none items-center justify-center">
        <Providers>
          <main className="flex flex-1 w-full">{children}</main>
        </Providers>
      </body>
    </html>
  );
}
