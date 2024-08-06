"use client";

import type React from "react";
import { motion } from "framer-motion";

import { useWizard, WizardContainer } from "@leftcurve/hooks";
import { Button, Input } from "@leftcurve/components";
import { useState } from "react";

interface Props {
	setSelectedOption: (option: "register" | "login") => void;
}

const Login: React.FC<Props> = ({ setSelectedOption }) => {
	return (
		<WizardContainer>
			<Step1 />
			<Step2 />
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
				<h1 className="text-xl font-bold font-grotesk">Log in</h1>
				<p className="text-xl font-grotesk">to enter in your account</p>
			</div>
			<div className="flex flex-col w-full gap-3">
				<Input />
				<Button onClick={nextStep}>Next</Button>
			</div>
			<div className="flex gap-1 start text-sm w-full">
				<p>No account?</p>
				<Button className="text-sm " variant="link" size="none">
					Sign up
				</Button>
			</div>
		</motion.div>
	);
};

const Step2: React.FC = () => {
	const { nextStep, previousStep, data } = useWizard();
	return (
		<motion.div
			className="flex flex-col gap-10 items-center w-full"
			initial={{ opacity: 0, translateX: 100 }}
			animate={{ opacity: 1, translateX: 0 }}
			exit={{ opacity: 0, translateX: -100 }}
		>
			<div className="flex flex-col gap-2 items-center justify-center font-bold font-grotesk text-xl">
				<p>
					Welcome back, <span className="text-primary-500">{data.name}</span>
				</p>
				<p>Login with your credential</p>
			</div>
			<div className="flex flex-col w-full gap-3">
				<Button color="primary">Login with Passkey</Button>
				<Button color="primary" variant="flat">
					Backpack
				</Button>
			</div>
			<div className=" w-full">
				<div className="flex gap-1 start text-sm">
					<p>No account?</p>
					<Button className="text-sm " variant="link" size="none">
						Sign up
					</Button>
				</div>
				<div className="flex gap-1 start text-sm w-full">
					<p>Not {data.name}?</p>
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

export default Login;
