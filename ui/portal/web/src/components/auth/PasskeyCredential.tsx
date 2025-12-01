import { Button, type ButtonProps, IconKey } from "@left-curve/applets-kit";

import { useMutation } from "@tanstack/react-query";

import type React from "react";

type PasskeyCredentialProps = {
  onAuth: () => Promise<void>;
  label: string;
  variant?: ButtonProps["variant"];
};

export const PasskeyCredential: React.FC<PasskeyCredentialProps> = ({ onAuth, label, variant }) => {
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
      variant={variant || "secondary"}
    >
      <IconKey className="w-6 h-6" />
      <p>{label}</p>
    </Button>
  );
};
