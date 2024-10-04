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
      <footer className="flex flex-col gap-10 items-center justify-center">
        <div className="flex gap-12 uppercase font-extrabold">
          <a href="/">X</a>
          <a href="/">DISCORD</a>
        </div>
        <div className="flex items-center justify-between md:justify-center text-xs font-light md:gap-12 px-4 w-full">
          <a href="/">TERMS OF USE</a>
          <a href="/">COOKIE POLICY</a>
          <a href="/">PRIVACY POLICY</a>
        </div>
      </footer>
    </div>
  );
}
