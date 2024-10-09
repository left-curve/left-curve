import { Footer } from "~/components/Footer";

export default function AuthLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <div className="min-h-full w-full flex-1 flex flex-col justify-center z-10 relative p-4">
      <a
        className="header w-full fixed h-[160px] top-0 py-4 md:py-7 px-6 md:px-12 z-50 left-0"
        href="/"
      >
        <img src="/images/logo.webp" alt="logo" className="h-6 md:h-[31px] object-contain" />
      </a>
      <div className="flex flex-1 w-full items-center justify-center">{children}</div>
      <Footer />
    </div>
  );
}
