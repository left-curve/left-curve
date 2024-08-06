"use client";

import type React from "react";
import { motion } from "framer-motion";

import { useWizard, WizardContainer } from "@leftcurve/hooks";
import { Button, Input } from "@leftcurve/components";
import { signWithCredential } from "@leftcurve/crypto";
import { useState } from "react";

interface Props {
	onFinish?: () => void;
	changeSelection: (selection: "register" | "login" | null) => void;
}

const LoginFlow: React.FC<Props> = ({ changeSelection, onFinish }) => {
	return (
		<WizardContainer
			onReset={() => changeSelection("register")}
			onFinish={onFinish}
		>
			<Step1 />
			<Step2 />
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
				<h1 className="text-xl font-bold font-grotesk">Log in</h1>
				<p className="text-xl font-grotesk">to enter in your account</p>
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
				<p>No account?</p>
				<Button className="text-sm " variant="link" size="none" onClick={reset}>
					Sign up
				</Button>
			</div>
		</motion.div>
	);
};

const Step2: React.FC = () => {
	const { previousStep, data, done } = useWizard<{
		userId: string;
	}>();

	const loginWithPasskey = async () => {
		const { signature } = await signWithCredential({
			rpId: window.location.hostname,
			hash: "0x01",
			userVerification: "preferred",
		});
		done();
	};

	const loginWithExternalWallet = async (wallet: string) => {
		done();
	};

	return (
		<motion.div
			className="flex flex-col gap-10 items-center w-full"
			initial={{ opacity: 0, translateX: 100 }}
			animate={{ opacity: 1, translateX: 0 }}
			exit={{ opacity: 0, translateX: -100 }}
		>
			<div className="flex flex-col gap-2 items-center justify-center font-bold font-grotesk text-xl">
				<p>
					Welcome back, <span className="text-primary-500">{data.userId}</span>
				</p>
				<p>Login with your credential</p>
			</div>
			<div className="flex flex-col w-full gap-3">
				<Button color="primary" onClick={loginWithPasskey}>
					Login with Passkey
				</Button>
				<Button
					color="primary"
					variant="flat"
					onClick={() => loginWithExternalWallet("backpack")}
				>
					Backpack
				</Button>
				<Button
					color="primary"
					variant="flat"
					onClick={() => loginWithExternalWallet("metamask")}
				>
					Metamask
				</Button>
				<Button
					color="primary"
					variant="flat"
					onClick={() => loginWithExternalWallet("phantom")}
				>
					Phantom
				</Button>
			</div>
			<div className=" w-full">
				<div className="flex gap-1 start text-sm w-full">
					<p>Not {data.userId}?</p>
					<Button
						className="text-sm "
						variant="link"
						size="none"
						onClick={previousStep}
					>
						Switch ID
					</Button>
				</div>
			</div>
		</motion.div>
	);
};

export default LoginFlow;
