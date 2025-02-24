import {
  IconAddCross,
  IconCopy,
  IconTrash,
  Spinner,
  TruncateText,
  twMerge,
  useSigningClient,
} from "@left-curve/applets-kit";
import { useAccount } from "@left-curve/store-react";
import { ConnectionStatus } from "@left-curve/store-react/types";
import { useQuery } from "@tanstack/react-query";
import type React from "react";

const KeyTranslation = {
  secp256r1: "Passkey",
  secp256k1: "Wallet",
};

export const KeyManagment: React.FC = () => {
  const { status, username, keyHash: currentKeyHash } = useAccount();
  const { data: signingClient } = useSigningClient();

  if (status !== ConnectionStatus.Connected) return null;

  const { data: keys = [], isPending } = useQuery({
    enabled: !!signingClient,
    queryKey: ["user_keys", username],
    queryFn: async () => await signingClient?.getKeysByUsername({ username }),
  });

  return (
    <div className="rounded-xl bg-rice-25 shadow-card-shadow flex flex-col w-full p-4 gap-4">
      <div className="flex flex-col md:flex-row gap-4 items-start justify-between">
        <div className="flex flex-col gap-1 max-w-lg">
          <h3 className="text-lg font-bold">Key Management </h3>
          <p className="text-gray-500 text-sm">
            Easily add or delete passkeys to manage access to your account. Each passkey is vital
            for secure logins, ensuring only you can access your account.
          </p>
        </div>
        <button
          type="button"
          className="w-full md:w-fit h-fit [box-shadow:0px_0px_8px_-2px_#FFFFFFA3_inset,_0px_3px_6px_-2px_#FFFFFFA3_inset,_0px_4px_6px_0px_#0000000A,_0px_4px_6px_0px_#0000000A] border-[1px] border-solid [border-image-source:linear-gradient(180deg,_rgba(46,_37,_33,_0.12)_8%,_rgba(46,_37,_33,_0.24)_100%)] bg-red-bean-400 px-6 py-2 rounded-full font-exposure text-red-bean-50 italic font-medium flex gap-2 items-center justify-center"
        >
          <IconAddCross className="w-5 h-5" />
          Add
        </button>
      </div>
      {isPending ? (
        <Spinner color="gray" size="md" />
      ) : (
        Object.entries(keys).map(([keyHash, key]) => {
          return (
            <div
              key={crypto.randomUUID()}
              className="flex items-center justify-between rounded-2xl border border-rice-200 hover:bg-rice-50 transition-all p-4"
            >
              <div className="flex items-start justify-between w-full gap-8">
                <div className="min-w-0">
                  <TruncateText className="text-gray-700 font-bold" text={keyHash} />

                  <p className="text-gray-500 text-sm">
                    {KeyTranslation[Object.keys(key).at(0) as keyof typeof KeyTranslation]}
                  </p>
                </div>
                <div className="flex gap-1">
                  <IconCopy className="w-5 h-5 cursor-pointer" copyText={keyHash} />
                  <IconTrash
                    className={twMerge("w-5 h-5 cursor-pointer", {
                      "text-gray-300 cursor-default": keyHash === currentKeyHash,
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
