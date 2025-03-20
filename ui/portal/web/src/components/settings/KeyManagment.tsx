import {
  Button,
  IconAddCross,
  IconCopy,
  IconTrash,
  Spinner,
  TruncateText,
  twMerge,
} from "@left-curve/applets-kit";
import { useAccount, useSigningClient } from "@left-curve/store-react";
import { ConnectionStatus } from "@left-curve/store-react/types";
import { useQuery } from "@tanstack/react-query";
import type React from "react";
import { useApp } from "~/hooks/useApp";
import { m } from "~/paraglide/messages";
import { Modals } from "../foundation/Modal";

const KeyTranslation = {
  secp256r1: "Passkey",
  secp256k1: "Wallet",
};

export const KeyManagment: React.FC = () => {
  const { status, username, keyHash: currentKeyHash } = useAccount();
  const { data: signingClient } = useSigningClient();
  const { showModal } = useApp();

  const { data: keys = [], isPending } = useQuery({
    enabled: !!signingClient && !!username,
    queryKey: ["user_keys", username],
    queryFn: async () => await signingClient?.getKeysByUsername({ username: username as string }),
  });

  if (status !== ConnectionStatus.Connected) return null;

  return (
    <div className="rounded-xl bg-rice-25 shadow-card-shadow flex flex-col w-full p-4 gap-4">
      <div className="flex flex-col md:flex-row gap-4 items-start justify-between">
        <div className="flex flex-col gap-1 max-w-lg">
          <h3 className="h4-bold text-gray-900">{m["settings.keyManagment.title"]()}</h3>
          <p className="text-gray-500 diatype-sm-regular">
            {m["settings.keyManagment.description"]()}
          </p>
        </div>
        <Button size="md" className="min-w-[120px]" onClick={() => showModal(Modals.AddKey)}>
          <IconAddCross className="w-5 h-5" />
          {m["settings.keyManagment.add"]()}
        </Button>
      </div>
      {isPending ? (
        <Spinner color="gray" size="md" />
      ) : (
        Object.entries(keys).map(([keyHash, key]) => {
          const isActive = keyHash === currentKeyHash;
          return (
            <div
              key={crypto.randomUUID()}
              className="flex items-center justify-between rounded-2xl border border-rice-200 hover:bg-rice-50 transition-all p-4"
            >
              <div className="flex items-start justify-between w-full gap-8">
                <div className="min-w-0">
                  <div className="flex gap-[6px] items-center">
                    <TruncateText className="text-gray-700 diatype-m-bold" text={keyHash} />
                    {isActive ? <span className="bg-status-success rounded-full h-2 w-2" /> : null}
                  </div>

                  <p className="text-gray-500 diatype-sm-medium">
                    {KeyTranslation[Object.keys(key).at(0) as keyof typeof KeyTranslation]}
                  </p>
                </div>
                <div className="flex gap-1">
                  <IconCopy className="w-5 h-5 cursor-pointer" copyText={keyHash} />
                  <IconTrash
                    onClick={() => (isActive ? null : showModal(Modals.RemoveKey, { keyHash }))}
                    className={twMerge("w-5 h-5 cursor-pointer", {
                      "text-gray-300 cursor-default": isActive,
                    })}
                  />
                </div>
              </div>
            </div>
          );
        })
      )}
    </div>
  );
};
