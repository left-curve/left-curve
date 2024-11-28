import type { Metadata } from "next";
import { Providers } from "./providers";

import "../public/styles/globals.css";

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
      <body>
        <Providers>{children}</Providers>
      </body>
    </html>
  );
}
