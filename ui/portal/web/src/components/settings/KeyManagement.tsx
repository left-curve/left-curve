import {
  Button,
  IconAddCross,
  IconTrash,
  Spinner,
  TextCopy,
  TruncateText,
  twMerge,
  useMediaQuery,
} from "@left-curve/applets-kit";
import { decodeBase64, encodeHex } from "@left-curve/dango/encoding";
import { uid } from "@left-curve/dango/utils";
import { useAccount, useSigningClient } from "@left-curve/store";
import { ConnectionStatus } from "@left-curve/store/types";
import { useQuery } from "@tanstack/react-query";
import type React from "react";
import { useApp } from "~/hooks/useApp";
import { m } from "~/paraglide/messages";
import { Modals } from "../foundation/RootModal";

const KeyTranslation = {
  secp256r1: "Passkey",
  secp256k1: "Wallet",
  ethereum: "Ethereum Wallet",
};

export const KeyManagement: React.FC = () => {
  const { status, username, keyHash: currentKeyHash } = useAccount();
  const { data: signingClient } = useSigningClient();
  const { showModal } = useApp();
  const { isMd } = useMediaQuery();

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
          <h3 className="h4-bold text-gray-900">{m["settings.keyManagement.title"]()}</h3>
          <p className="text-gray-500 diatype-sm-regular">
            {m["settings.keyManagement.description"]()}
          </p>
        </div>
        <Button size="md" className="min-w-[120px]" onClick={() => showModal(Modals.AddKey)}>
          <IconAddCross className="w-5 h-5" />
          {m["settings.keyManagement.add"]()}
        </Button>
      </div>
      {isPending ? (
        <Spinner color="gray" size="md" />
      ) : (
        Object.entries(keys).map(([keyHash, key]) => {
          const isActive = keyHash === currentKeyHash;
          const [[keyType, keyValue]] = Object.entries(key);
          const isEthereumKey = keyType === "ethereum";
          const keyRepresentation = isEthereumKey
            ? keyValue
            : `0x${encodeHex(decodeBase64(keyValue))}`;

          return (
            <div
              key={uid()}
              className="flex items-center justify-between rounded-2xl border border-rice-200 hover:bg-rice-50 transition-all p-4"
            >
              <div className="flex items-start justify-between w-full gap-8">
                <div className="min-w-0">
                  <div className="flex gap-[6px] items-center text-gray-700 diatype-m-bold">
                    {isMd ? <p>{keyRepresentation}</p> : <TruncateText text={keyRepresentation} />}
                    {isActive ? <span className="bg-status-success rounded-full h-2 w-2" /> : null}
                  </div>

                  <p className="text-gray-500 diatype-sm-medium">
                    {KeyTranslation[keyType as keyof typeof KeyTranslation]}
                  </p>
                </div>
                <div className="flex gap-1">
                  <TextCopy className="w-5 h-5 cursor-pointer" copyText={keyRepresentation} />
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
