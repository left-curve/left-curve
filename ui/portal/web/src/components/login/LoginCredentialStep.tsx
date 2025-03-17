import { useMediaQuery, useWizard } from "@left-curve/applets-kit";
import { useLogin } from "@left-curve/store-react";
import { useNavigate } from "@tanstack/react-router";
import { useToast } from "../Toast";

import { Button, IconLeft } from "@left-curve/applets-kit";
import { AuthOptions } from "../auth/AuthOptions";

import { m } from "~/paraglide/messages";

import type React from "react";
import { AuthMobile } from "../auth/AuthMobile";

export const LoginCredentialStep: React.FC = () => {
  const navigate = useNavigate();
  const { toast } = useToast();
  const { data, previousStep } = useWizard<{ username: string }>();
  const isMd = useMediaQuery("md");

  const { username } = data;

  const { mutateAsync: connectWithConnector, isPending } = useLogin({
    username,
    mutation: {
      onSuccess: () => navigate({ to: "/" }),
      onError: (err) => {
        console.error(err);
        toast.error({
          title: m["common.error"](),
          description: m["login.errors.failedSigingIn"](),
        });
        previousStep();
      },
    },
  });

  return (
    <>
      <div className="flex items-center justify-center flex-col gap-8 px-4 lg:px-0">
        <div className="flex flex-col gap-7 items-center justify-center">
          <img
            src="./favicon.svg"
            alt="dango-logo"
            className="h-12 rounded-full shadow-btn-shadow-gradient"
          />
          <div className="flex flex-col gap-3 items-center justify-center text-center">
            <h1 className="h2-heavy">
              {m["common.hi"]()}, {username}
            </h1>
            <p className="text-gray-500 diatype-m-medium">{m["login.credential.description"]()}</p>
          </div>
        </div>
        {isMd ? (
          <AuthOptions
            action={(connectorId) => connectWithConnector({ connectorId })}
            isPending={isPending}
            mode="signin"
          />
        ) : (
          <AuthMobile />
        )}
        <div className="flex items-center">
          <Button variant="link" onClick={() => previousStep()}>
            <IconLeft className="w-[22px] h-[22px] text-blue-500" />
            <p className="leading-none pt-[2px]">{m["common.back"]()}</p>
          </Button>
        </div>
      </div>
    </>
  );
};
