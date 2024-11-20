import { Header } from "../components/Header";

export default function PortalLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <div className="flex flex-col min-h-screen w-full h-full bg-surface-off-white-200 relative scrollbar-none items-center justify-center">
      <img
        src="/images/background.png"
        alt="bg-image"
        className="object-cover drag-none select-none h-[80vh] absolute top-[15%] left-1/2 transform -translate-x-1/2 z-0 blur-2xl opacity-40"
      />

      <Header />
      <main className="flex flex-1 w-full z-[2]">{children}</main>
    </div>
  );
}
