"use client";

import type React from "react";
import { motion } from "framer-motion";

import { useWizard, WizardContainer } from "@leftcurve/hooks";
import { BackArrow, Button, Input } from "@leftcurve/components";
import { useState } from "react";

interface Props {
	setSelectedOption: (option: "register" | "login") => void;
}

const Register: React.FC<Props> = ({ setSelectedOption }) => {
	return (
		<WizardContainer>
			<Step1 />
			<Step2 />
			<Step3 />
		</WizardContainer>
	);
};

const Step1: React.FC = () => {
	const { nextStep, handleStep } = useWizard();
	const [test, setTest] = useState("test");

	handleStep(
		() => {
			setTest("blueblue");
		},
		{ name: test },
	);

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
				<Input placeholder="jonh-doe" />
				<Button onClick={nextStep}>Next</Button>
			</div>
			<div className="flex gap-1 start text-sm w-full">
				<p>You already have an account?</p>
				<Button className="text-sm " variant="link" size="none">
					Sign in
				</Button>
			</div>
		</motion.div>
	);
};

const Step2: React.FC = () => {
	const { nextStep, previousStep, data } = useWizard();

	return (
		<motion.div
			className="flex flex-col gap-10 items-center w-full relative"
			initial={{ opacity: 0, translateX: 100 }}
			animate={{ opacity: 1, translateX: 0 }}
			exit={{ opacity: 0, translateX: -100 }}
		>
			<div className="flex flex-col gap-2 items-center justify-center font-bold font-grotesk text-xl">
				<p>
					Welcome, <span className="text-primary-500">{data.name}</span>
				</p>
				<p>Choose a credential to register</p>
			</div>
			<div className="flex flex-col w-full gap-3">
				<Button color="primary" onClick={nextStep}>
					Passkey
				</Button>
				<Button color="primary" variant="flat" onClick={nextStep}>
					Metamask
				</Button>
				<Button color="primary" variant="flat" onClick={nextStep}>
					Phantom
				</Button>
				<Button color="primary" variant="flat" onClick={nextStep}>
					Backpack
				</Button>
			</div>
			<div className="flex gap-1 start text-sm w-full">
				<p>You already have an account?</p>
				<Button className="text-sm " variant="link" size="none">
					Sign in
				</Button>
			</div>
		</motion.div>
	);
};

const Step3: React.FC = () => {
	const { nextStep, previousStep, data } = useWizard();

	return (
		<motion.div
			className="flex flex-col gap-10 items-center w-full relative"
			initial={{ opacity: 0, translateX: 100 }}
			animate={{ opacity: 1, translateX: 0 }}
			exit={{ opacity: 0, translateX: -100 }}
		>
			<div className="flex flex-col gap-2 items-center justify-center font-bold font-grotesk text-xl">
				<p>
					One more step, <span className="text-primary-500">{data.name}</span>
				</p>
				<p>Choose an account type</p>
			</div>
			<div className="flex flex-col w-full gap-3">
				<Button color="primary" onClick={nextStep}>
					Spot account
				</Button>
				<Button color="primary" onClick={nextStep}>
					Margin account
				</Button>
			</div>
			{/* <div className="flex gap-1 start text-sm w-full">
				<p>You already have an account?</p>
				<Button className="text-sm " variant="link" size="none">
					Sign in
				</Button>
			</div> */}
		</motion.div>
	);
};

export default Register;
