import { useAccount, useConfig, useSessionKey } from "@left-curve/store";

import {
  CurrentBlock,
  IconEdit,
  IconMobile,
  IconNetwork,
  IconTimer,
  IconUser,
  Modals,
  useApp,
  useMediaQuery,
} from "@left-curve/applets-kit";

import { SessionCountdown } from "./SessionCountdown";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";
import type { PropsWithChildren } from "react";

const Container: React.FC<PropsWithChildren> = ({ children }) => {
  return (
    <div className="rounded-xl bg-surface-secondary-rice shadow-account-card flex flex-col w-full px-2 py-4 gap-4">
      <h3 className="h4-bold text-ink-primary-900 px-2">{m["settings.session.title"]()}</h3>
      {children}
    </div>
  );
};

const UsernameSection: React.FC = () => {
  const { username, isConnected, account } = useAccount();
  const { showModal } = useApp();

  const hasntChangeUsername = username === `User #${account?.index}`;

  if (!isConnected) return null;

  return (
    <div className="flex items-center justify-between rounded-md gap-8 px-2">
      <div className="flex flex-col">
        <div className="flex items-start gap-2">
          <IconUser className="text-ink-tertiary-500" />
          <p className="diatype-m-bold text-ink-secondary-700">{m["common.username"]()}</p>
        </div>
      </div>
      <div
        className="text-ink-secondary-700 px-4 py-3 md:shadow-account-card rounded-md min-w-[9rem] h-[46px] flex items-center justify-end md:justify-center gap-1"
        onClick={() => hasntChangeUsername && showModal(Modals.EditUsername)}
      >
        <p>{username}</p>
        {hasntChangeUsername && <IconEdit className="w-5 h-5" />}
      </div>
    </div>
  );
};

const RemainingTimeSection: React.FC = () => {
  const { session } = useSessionKey();
  if (!session) return null;

  return (
    <div className="flex items-start justify-between rounded-md gap-8 px-2">
      <div className="flex flex-col gap-2 md:gap-0 w-full">
        <div className="flex justify-between items-center gap-2">
          <div className="flex gap-2 items-center">
            <IconTimer className="text-ink-tertiary-500" />
            <span className="diatype-m-bold text-ink-secondary-700 capitalize">
              {m["settings.session.remaining"]()}
            </span>
          </div>
          <SessionCountdown />
        </div>

        <p className="text-ink-tertiary-500 diatype-sm-regular pl-8 max-w-lg">
          {m["settings.session.description"]()}
        </p>
      </div>
    </div>
  );
};

const NetworkSection: React.FC = () => {
  const { chain } = useConfig();

  return (
    <div className="flex items-start justify-between rounded-md gap-8 w-full px-2">
      <div className="flex flex-col gap-2 md:gap-0 w-full">
        <div className="flex justify-between items-center gap-2 capitalize">
          <div className="flex gap-2 items-center">
            <IconNetwork className="text-ink-tertiary-500" />
            <span className="diatype-m-bold text-ink-secondary-700">
              {m["settings.session.network.title"]()}
            </span>
          </div>
          <div className="text-ink-secondary-700 px-4 py-3 md:shadow-account-card rounded-md min-w-[9rem] h-[46px] flex items-center justify-end md:justify-center">
            {chain.name}
          </div>
        </div>

        <div className="flex flex-col  rounded-md justify-center gap-1 w-fit md:gap-0 pl-8">
          <div className="flex md:items-center flex-col md:flex-row diatype-sm-regular">
            <p className="md:min-w-[10rem] text-ink-tertiary-500">
              {m["settings.session.network.latestBlockHeight"]()}
            </p>
            <CurrentBlock
              classNames={{ container: "break-all whitespace-normal", skeleton: "h-4 w-24" }}
              selector="height"
            />
          </div>

          <div className="flex md:items-center flex-col md:flex-row diatype-sm-regular">
            <p className="md:min-w-[10rem] text-ink-tertiary-500">
              {m["settings.session.network.latestBlockTime"]()}
            </p>
            <CurrentBlock
              classNames={{ container: "break-all whitespace-normal", skeleton: "h-4 w-24" }}
              selector="timestamp"
            />
          </div>

          <div className="flex md:items-center flex-col md:flex-row diatype-sm-regular">
            <p className="md:min-w-[10rem] text-ink-tertiary-500">
              {m["settings.session.network.endpoint"]()}
            </p>
            <p className="break-all whitespace-normal">
              {chain.urls.indexer.replace(/\/graphql$/, "")}
            </p>
          </div>
        </div>
      </div>
    </div>
  );
};

const ConnectMobileSection: React.FC = () => {
  const { showModal } = useApp();
  const { isConnected } = useAccount();
  const { isLg } = useMediaQuery();
  const { session } = useSessionKey();

  if ((!isConnected && !isLg) || !session) return null;

  return (
    <div className="flex w-full pr-2">
      <button
        type="button"
        className="flex items-center justify-between pl-2 py-4 rounded-md hover:bg-surface-tertiary-rice transition-all cursor-pointer w-full"
        onClick={() => showModal(Modals.QRConnect)}
      >
        <span className="flex items-center justify-center gap-2">
          <IconMobile className="text-ink-tertiary-500" />
          <span className="diatype-m-bold text-ink-secondary-700">
            {m["settings.connectToMobile"]()}
          </span>
        </span>
      </button>
    </div>
  );
};

export const SessionSection = Object.assign(Container, {
  Username: UsernameSection,
  RemainingTime: RemainingTimeSection,
  Network: NetworkSection,
  ConnectMobile: ConnectMobileSection,
});
