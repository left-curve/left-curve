import { Button, IconApple, IconGoogle } from "@left-curve/applets-kit";
import { wait } from "@left-curve/dango/utils";
import { useLoginWithOAuth, usePrivy } from "@privy-io/react-auth";
import { useMutation } from "@tanstack/react-query";

type SocialCredentialProps = {
  signup?: boolean;
  onAuth: () => Promise<void>;
};

export const SocialCredential: React.FC<SocialCredentialProps> = ({ onAuth, signup }) => {
  const { createWallet } = usePrivy();
  const { initOAuth, loading } = useLoginWithOAuth({
    onComplete: () => onComplete.mutateAsync(),
  });

  const onComplete = useMutation({
    mutationFn: async () => {
      if (signup) await createWallet();
      await wait(500);
      await onAuth();
    },
  });

  const onClick = useMutation({
    mutationFn: async () => {
      if ((window as any).privy) await onAuth();
      else initOAuth({ provider: "google", disableSignup: !signup });
    },
  });

  return (
    <div className="grid grid-cols-2 gap-3 w-full">
      <Button
        onClick={() => onClick.mutateAsync()}
        variant="secondary"
        fullWidth
        isLoading={loading || onComplete.isPending || onClick.isPending}
      >
        <IconGoogle />
      </Button>
      <Button isDisabled={true} variant="secondary" fullWidth>
        <IconApple />
      </Button>
    </div>
  );
};
