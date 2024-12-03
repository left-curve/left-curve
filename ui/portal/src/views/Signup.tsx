import { motion } from "framer-motion";
import { WizardSignup } from "~/components/WizardSignup";

const Signup: React.FC = () => {
  return (
    <motion.div
      className="flex flex-1 w-full h-full"
      animate={{ height: "auto" }}
      initial={{ height: 0 }}
      transition={{ duration: 0.5 }}
    >
      <WizardSignup />
    </motion.div>
  );
};

export default Signup;
