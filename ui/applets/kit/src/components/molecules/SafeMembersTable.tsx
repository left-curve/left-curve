import { AnimatePresence, motion } from "framer-motion";

import { SafeMemberRow } from "./SafeMemberRow";

import type { Account, AccountType } from "@left-curve/dango/types";
import { Button } from "../atoms/Button";

interface Props {
  account: Account<typeof AccountType.Safe>;
}

export const SafeMembersTable: React.FC<Props> = ({ account }) => {
  const { members } = account.params.safe;

  const totalPower = Object.values(members).reduce((acc, power) => acc + power, 0);

  return (
    <AnimatePresence key={crypto.randomUUID()} mode="wait">
      <motion.div
        initial={{ opacity: 0, translateY: 100 }}
        animate={{ opacity: 1, translateY: 0 }}
        exit={{ opacity: 0, translateY: 100 }}
        className="flex items-center justify-center gap-4 flex-col"
      >
        <div className="bg-surface-yellow-100 rounded-2xl w-full p-3 md:p-4 flex flex-col gap-6 ">
          <div className="flex flex-col gap-2 text-typography-black">
            <div className="p-2 md:p-4 font-extrabold text-sand-800/50 font-diatype-rounded tracking-widest uppercase flex items-center justify-between gap-2">
              <p>Name</p>
              <p>Vote Weight</p>
            </div>
            {Object.entries(members).map(([username, power]) => (
              <SafeMemberRow
                key={username}
                username={username}
                power={power}
                totalPower={totalPower}
              />
            ))}
          </div>
          <Button className="flex-1 min-h-11">New Member</Button>
        </div>
      </motion.div>
    </AnimatePresence>
  );
};
