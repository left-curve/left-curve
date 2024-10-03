"use client";

import { DangoButton, Input, useWizard } from "@dango/shared";
import { usePublicClient } from "@leftcurve/react";
import Link from "next/link";
import type React from "react";
import { useForm } from "react-hook-form";

export const LoginStep: React.FC = () => {
  const { nextStep, setData, previousStep, data } = useWizard();
  const { setError, register, watch, formState } = useForm<{ username: string; retry: boolean }>({
    mode: "onChange",
  });
  const client = usePublicClient();
  const username = watch("username");

  const { retry } = data;
  const { errors } = formState;

  const onSubmit = async () => {
    if (!username) return;
    const { accounts } = await client.getUser({ username });
    const numberOfAccounts = Object.keys(accounts).length;
    if (numberOfAccounts === 0) {
      setError("username", { message: "Username doesn't exist" });
    } else {
      setData({ username, retry: false });
      nextStep();
    }
  };
  return (
    <>
      {retry ? (
        <p className="text-typography-rose-600 text-center text-xl">
          The credential connected does not match the on-chain record.
          <br /> Please try again.
        </p>
      ) : null}
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
          {retry ? "Choose credentials" : "Login"}
        </DangoButton>
        {retry ? (
          <DangoButton
            onClick={() => [previousStep(), setData({ username, retry: false })]}
            variant="ghost"
            color="sand"
            className="text-lg"
          >
            Back
          </DangoButton>
        ) : (
          <DangoButton as={Link} href="/signup" variant="ghost" color="sand" className="text-lg">
            Don't have an account?
          </DangoButton>
        )}
      </div>
    </>
  );
};
