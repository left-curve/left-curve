"use client";
import { AccountConnect } from "@leftcurve/applets";
import { useRouter } from "next/navigation";

export default function Home() {
	const { push: goToPage } = useRouter();
	return (
		<main className="flex min-h-screen items-center justify-center">
			<AccountConnect onFinish={() => goToPage("/profile")} />
		</main>
	);
}
