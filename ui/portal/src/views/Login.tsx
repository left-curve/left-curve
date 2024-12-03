import { motion } from "framer-motion";
import { WizardLogin } from "~/components/WizardLogin";

const Login: React.FC = () => {
  return (
    <motion.div
      className="flex flex-1 w-full h-full"
      animate={{ height: "auto" }}
      initial={{ height: 0 }}
      transition={{ duration: 0.5 }}
    >
      <WizardLogin />
    </motion.div>
  );
};

export default Login;
