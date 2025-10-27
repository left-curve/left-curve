import { useEffect } from "react";
import { useMutation } from "@tanstack/react-query";
import { useConnectors } from "@left-curve/store";

import { Button, IconGoogle, IconTwitter, useApp } from "@left-curve/applets-kit";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { PRIVY_ERRORS_MAPPING } from "~/constants";

import type { Connector } from "@left-curve/store/types";
import type Privy from "@privy-io/js-sdk-core";
import type { OAuthProviderType } from "@privy-io/js-sdk-core";

type SocialCredentialProps = {
  signup?: boolean;
  onAuth: () => Promise<void>;
};

export const SocialCredential: React.FC<SocialCredentialProps> = ({ onAuth, signup }) => {
  const { toast } = useApp();
  const connectors = useConnectors();
  const connector = connectors.find((c) => c.id === "privy") as Connector & { privy: Privy };

  const requestAuth = useMutation({
    mutationFn: async (provider: OAuthProviderType) => {
      const { url } = await connector.privy.auth.oauth.generateURL(
        provider,
        `${window.location.origin}/${signup ? "signup" : "signin"}`,
      );
      window.location.href = url;
    },
  });

  const handleAuth = useMutation({
    mutationFn: async (params: { code: string; status: string; provider: OAuthProviderType }) => {
      const { code, status, provider } = params;
      await connector.privy.auth.oauth.loginWithCode(
        code,
        status,
        provider,
        undefined,
        signup ? "login-or-sign-up" : "no-signup",
      );

      await onAuth();
    },
    onError: (e) => {
      const message = "message" in (e as object) ? (e as Error).message : "authFailed";
      const error =
        PRIVY_ERRORS_MAPPING[message as keyof typeof PRIVY_ERRORS_MAPPING] ||
        m["auth.errors.authFailed"]();
      toast.error({
        title: m["common.error"](),
        description: error,
      });
    },
  });

  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const oauthState = params.get("privy_oauth_state");
    const oauthCode = params.get("privy_oauth_code");
    const oauthProvider = params.get("privy_oauth_provider");
    if (!oauthCode || !oauthState || !oauthProvider) return;
    handleAuth.mutate({
      code: oauthCode,
      status: oauthState,
      provider: oauthProvider as OAuthProviderType,
    });
  }, []);

  return (
    <div className="grid grid-cols-2 gap-3 w-full">
      <Button
        onClick={() => requestAuth.mutateAsync("google")}
        variant="secondary"
        fullWidth
        isLoading={
          (requestAuth.isPending && requestAuth.variables === "google") ||
          (handleAuth.isPending && handleAuth.variables?.provider === "google")
        }
      >
        <IconGoogle />
      </Button>
      <Button
        isDisabled
        onClick={() => requestAuth.mutateAsync("twitter")}
        variant="secondary"
        fullWidth
        isLoading={
          (requestAuth.isPending && requestAuth.variables === "twitter") ||
          (handleAuth.isPending && handleAuth.variables?.provider === "twitter")
        }
      >
        <IconTwitter />
      </Button>
    </div>
  );
};
