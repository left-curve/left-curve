"use client";

import { createWebAuthnCredential } from "@leftcurve/crypto";
import { getNavigatorOS } from "@leftcurve/utils";
import { motion } from "framer-motion";
import { useState } from "react";
import { Button, Input } from "~/components";
import { WizardContainer, useWizard } from "~/hooks";

import type React from "react";

interface Props {
  onFinish?: () => void;
  changeSelection: (selection: "register" | "login" | null) => void;
}

const SignupFlow: React.FC<Props> = ({ changeSelection, onFinish }) => {
  return (
    <WizardContainer onReset={() => changeSelection("login")} onFinish={onFinish}>
      <Step1 />
      <Step2 />
      <Step3 />
    </WizardContainer>
  );
};

const Step1: React.FC = () => {
  const { nextStep, onStepLeave, setData, reset } = useWizard();
  const [userId, setUserId] = useState("");

  onStepLeave(() => {
    setData({ userId });
  });

  return (
    <motion.div
      className="flex flex-col gap-10 items-center w-full justify-between flex-1"
      initial={{ opacity: 0, translateX: 100 }}
      animate={{ opacity: 1, translateX: 0 }}
      exit={{ opacity: 0, translateX: -100 }}
    >
      <div className="flex flex-col gap-3 items-center justify-center">
        <h1 className="text-xl font-bold font-grotesk">Create your account</h1>
      </div>
      <div className="flex flex-col w-full gap-3">
        <Input
          placeholder="Account Id"
          onChange={({ target }) => setUserId(target.value)}
          value={userId}
        />
        <Button onClick={nextStep}>Next</Button>
      </div>
      <div className="flex gap-1 start text-sm w-full">
        <p>You already have an account?</p>
        <Button className="text-sm " variant="link" size="none" onClick={reset}>
          Sign in
        </Button>
      </div>
    </motion.div>
  );
};

const Step2: React.FC = () => {
  const { nextStep, previousStep, data, setData, reset } = useWizard();

  const getPublicKeyFromPasskey = async () => {
    try {
      const { id, publicKey } = await createWebAuthnCredential({
        user: {
          name: `${getNavigatorOS()} ${new Date().toLocaleString()}`,
        },
        rp: {
          name: window.document.title,
          id: window.location.hostname,
        },
        authenticatorSelection: {
          residentKey: "preferred",
          requireResidentKey: false,
          userVerification: "preferred",
        },
      });
      setData({ ...data, id, publicKey });
      nextStep();
    } catch (error) {}
  };

  const getPublickeyWithExternalWallet = (wallet: string) => {
    console.log("get public key with external wallet");
    nextStep();
  };

  return (
    <motion.div
      className="flex flex-col gap-10 items-center w-full relative"
      initial={{ opacity: 0, translateX: 100 }}
      animate={{ opacity: 1, translateX: 0 }}
      exit={{ opacity: 0, translateX: -100 }}
    >
      <div className="flex flex-col gap-2 items-center justify-center font-bold font-grotesk text-xl">
        <p>
          Welcome, <span className="text-primary-500">{data.userId}</span>
        </p>
        <p>Choose a credential to register</p>
      </div>
      <div className="flex flex-col w-full gap-3">
        <Button color="primary" onClick={getPublicKeyFromPasskey}>
          Passkey
        </Button>
        <Button
          color="primary"
          variant="flat"
          onClick={() => getPublickeyWithExternalWallet("metamask")}
        >
          Metamask
        </Button>
        <Button
          color="primary"
          variant="flat"
          onClick={() => getPublickeyWithExternalWallet("phantom")}
        >
          Phantom
        </Button>
        <Button
          color="primary"
          variant="flat"
          onClick={() => getPublickeyWithExternalWallet("backpack")}
        >
          Backpack
        </Button>
      </div>
      <div className="flex gap-1 start text-sm w-full">
        <p>You already have an account?</p>
        <Button className="text-sm " variant="link" size="none" onClick={reset}>
          Sign in
        </Button>
      </div>
    </motion.div>
  );
};

const Step3: React.FC = () => {
  const { done, data } = useWizard();

  const mockCreateAccount = () => {
    done();
  };

  return (
    <motion.div
      className="flex flex-col gap-10 items-center w-full relative"
      initial={{ opacity: 0, translateX: 100 }}
      animate={{ opacity: 1, translateX: 0 }}
      exit={{ opacity: 0, translateX: -100 }}
    >
      <div className="flex flex-col gap-2 items-center justify-center font-bold font-grotesk text-xl">
        <p>
          One more step, <span className="text-primary-500">{data.userId}</span>
        </p>
        <p>Choose an account type</p>
      </div>
      <div className="flex flex-col w-full gap-3">
        <Button color="primary" onClick={mockCreateAccount}>
          Spot account
        </Button>
        <Button color="primary" onClick={mockCreateAccount}>
          Margin account
        </Button>
      </div>
    </motion.div>
  );
};

export default SignupFlow;
