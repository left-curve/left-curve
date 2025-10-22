import { Button, IconGoogle, IconTwitter } from "@left-curve/applets-kit";
import { wait } from "@left-curve/dango/utils";
import { useMutation } from "@tanstack/react-query";
import { useState } from "react";

type SocialCredentialProps = {
  signup?: boolean;
  onAuth: () => Promise<void>;
};

export const SocialCredential: React.FC<SocialCredentialProps> = ({ onAuth, signup }) => {
  const [onAuthProvider, setOnAuthProvider] = useState<"google" | "twitter" | null>(null);
  /*   const { createWallet } = usePrivy();
  const { initOAuth } = useLoginWithOAuth({
    onComplete: ({ loginMethod }) => {
      setOnAuthProvider(loginMethod as string as "google" | "twitter");
      onComplete.mutateAsync();
    },
  }); */

  const onComplete = useMutation({
    mutationFn: async () => {
      // if (signup) await createWallet();
      await wait(500);
      await onAuth();
    },
  });

  const googleAuth = useMutation({
    mutationFn: async () => {
      if ((window as any).privy) await onAuth();
      // else await initOAuth({ provider: "google", disableSignup: !signup });
    },
  });

  const xAuth = useMutation({
    mutationFn: async () => {
      if ((window as any).privy) await onAuth();
      // else await initOAuth({ provider: "twitter", disableSignup: !signup });
    },
  });

  return (
    <div className="grid grid-cols-2 gap-3 w-full">
      <Button
        onClick={() => googleAuth.mutateAsync()}
        variant="secondary"
        fullWidth
        isLoading={(onAuthProvider === "google" && onComplete.isPending) || googleAuth.isPending}
      >
        <IconGoogle />
      </Button>
      <Button
        onClick={() => xAuth.mutateAsync()}
        variant="secondary"
        fullWidth
        isLoading={(onAuthProvider === "twitter" && onComplete.isPending) || xAuth.isPending}
      >
        <IconTwitter />
      </Button>
    </div>
  );
};
