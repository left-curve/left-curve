"use client";

import { useParams } from "next/navigation";
import { AccountRouter } from "./AccountRouter";

export default function AccountsPage() {
  const { address = "0" } = useParams<{ address: string }>();
  return (
    <div className="min-h-full w-full flex-1 flex justify-center z-10 relative p-4">
      <div className="flex flex-1 flex-col items-center justify-center gap-4 w-full md:max-w-2xl">
        <AccountRouter index={Number.parseInt(address)} />
      </div>
    </div>
  );
}
