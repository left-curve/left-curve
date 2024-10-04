export default function AuthLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <div className="min-h-full w-full flex-1 flex justify-center z-10 relative p-4">
      <div className="header w-full fixed h-[160px] top-0 py-7 px-12 z-50">
        <img src="/images/logo.webp" alt="logo" className="h-6 md:h-[31px] object-contain" />
      </div>
      <div className="flex flex-1 w-full items-center justify-center">{children}</div>
    </div>
  );
}
