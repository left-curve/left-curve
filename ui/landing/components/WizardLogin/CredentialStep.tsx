"use client";

import { DangoButton, Input, useWizard } from "@dango/shared";
import { usePublicClient } from "@leftcurve/react";
import Link from "next/link";
import type React from "react";
import { useForm } from "react-hook-form";

export const CredentialStep: React.FC = () => {
  const { nextStep, setData } = useWizard();
  const { setError, register, watch, formState } = useForm<{ username: string }>({
    mode: "onChange",
  });
  const client = usePublicClient();
  const username = watch("username");

  const { errors } = formState;

  const onSubmit = async () => {
    if (!username) return;
    setData({ username });
    const { accounts } = await client.getUser({ username });
    const numberOfAccounts = Object.keys(accounts).length;
    if (numberOfAccounts > 0) {
      setError("username", { message: "Username is already taken" });
    } else {
      nextStep();
    }
  };

  return (
    <>
      <Input
        {...register("username", {
          validate: (value) => {
            if (!value) return "Username is required";
            if (value.length > 15) return "Username must be at most 15 characters long";
            return true;
          },
        })}
        placeholder="Choose an username"
        onKeyDown={({ key }) => key === "Enter" && onSubmit()}
        error={errors.username?.message}
      />
      <div className="flex flex-col w-full gap-3 md:gap-6">
        <DangoButton fullWidth onClick={onSubmit}>
          Choose credentials
        </DangoButton>
        <DangoButton as={Link} href="/signup" variant="ghost" color="sand" className="text-lg">
          Already have an account?
        </DangoButton>
      </div>
    </>
  );
};
