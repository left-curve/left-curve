import { Button, IconPasskey } from "@left-curve/applets-kit";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useMutation } from "@tanstack/react-query";

import type React from "react";

type PasskeyCredentialProps = {
  action: "signin" | "signup";
  onAuth: () => Promise<void>;
};

export const PasskeyCredential: React.FC<PasskeyCredentialProps> = ({ onAuth, action }) => {
  const { isPending, mutateAsync } = useMutation({
    mutationFn: async () => {
      await onAuth();
    },
  });

  return (
    <Button
      fullWidth
      onClick={() => mutateAsync()}
      isLoading={isPending}
      className="gap-2"
      variant="secondary"
    >
      <IconPasskey className="w-6 h-6" />
      <p className="min-w-20"> {m["common.signWithPasskey"]({ action })}</p>
    </Button>
  );
};
