import { Providers } from "./providers";

import "../public/globals.css";
import "@dango/assets/fonts/ABCDiatypeRounded/index.css";
import "@dango/assets/fonts/Exposure/index.css";

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className={"antialiased"}>
        <Providers>{children}</Providers>
      </body>
    </html>
  );
}
