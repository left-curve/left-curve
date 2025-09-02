import { useMemo } from "react";

import { SearchItem } from "./SearchItem";
import { GlobalText } from "../foundation";
import { MotiView, AnimatePresence } from "moti";
import { View, Pressable, ScrollView } from "react-native";

import type React from "react";
import type { SearchBarResult } from "@left-curve/store";
import type { AppletMetadata } from "@left-curve/store/types";

const childAnim = {
  from: { opacity: 0, translateY: -30 },
  animate: { opacity: 1, translateY: 0 },
};

const Root: React.FC<React.PropsWithChildren> = ({ children }) => <>{children}</>;

type SearchMenuProps = {
  isSearching: boolean;
  isLoading: boolean;
  searchResult: SearchBarResult;
  allApplets: AppletMetadata[];
  onSelect: (path: string) => void;
};

const Body: React.FC<SearchMenuProps> = ({
  isSearching,
  isLoading,
  searchResult,
  allApplets,
  onSelect,
}) => {
  const { applets, block, txs, account, contract } = searchResult;

  const groups = useMemo(() => {
    const out: Array<{ key: string; title: string; items: React.ReactNode[] }> = [];

    if (applets.length) {
      out.push({
        key: "applets-fav-or-result",
        title: isSearching ? "Applets" : "Favorite Applets",
        items: applets.map((applet) => (
          <Pressable
            key={`applet-${applet.id}`}
            className="w-full"
            onPress={() => onSelect(applet.path)}
          >
            <SearchItem.Applet {...applet} />
          </Pressable>
        )),
      });
    }

    if (!isSearching && allApplets.length) {
      out.push({
        key: "applets-all",
        title: "Applets",
        items: allApplets.map((applet) => (
          <Pressable
            key={`applet-all-${applet.title}`}
            className="w-full"
            onPress={() => onSelect(applet.path)}
          >
            <SearchItem.Applet {...applet} />
          </Pressable>
        )),
      });
    }

    if (block) {
      out.push({
        key: "block",
        title: "Block",
        items: [
          <Pressable
            key={`block-${block.blockHeight}`}
            className="w-full"
            onPress={() => {
              onSelect(`/block/${block.blockHeight}`);
            }}
          >
            <SearchItem.Block height={block.blockHeight} hash={block.hash} />
          </Pressable>,
        ],
      });
    }

    if (txs.length) {
      out.push({
        key: "txs",
        title: "Transactions",
        items: txs.map((tx) => (
          <Pressable
            key={`tx-${tx.hash}`}
            className="w-full"
            onPress={() => {
              onSelect(`/tx/${tx.hash}`);
            }}
          >
            <SearchItem.Transaction height={tx.blockHeight} hash={tx.hash} />
          </Pressable>
        )),
      });
    }

    if (account) {
      out.push({
        key: "accounts",
        title: "Accounts",
        items: [
          <Pressable
            key={`acc-${account.address}`}
            className="w-full"
            onPress={() => {
              onSelect(`/account/${account.address}`);
            }}
          >
            <SearchItem.Account account={account} />
          </Pressable>,
        ],
      });
    }

    if (contract) {
      out.push({
        key: "contracts",
        title: "Contracts",
        items: [
          <Pressable
            key={`contract-${contract.address}`}
            className="w-full"
            onPress={() => {
              onSelect(`/contract/${contract.address}`);
            }}
          >
            <SearchItem.Contract contract={contract} />
          </Pressable>,
        ],
      });
    }

    return out;
  }, [applets, allApplets, block, txs, account, contract, onSelect, isSearching]);

  return (
    <AnimatePresence>
      <MotiView className="w-full overflow-hidden rounded-xs">
        <ScrollView
          className="w-full"
          contentContainerClassName="lg:p-1 w-full items-center gap-1"
          showsVerticalScrollIndicator
        >
          {isLoading ? (
            <View className="flex items-center justify-center w-full p-2">
              {/* TODO: Add Spinner */}
              <GlobalText>Searching...</GlobalText>
            </View>
          ) : groups.length === 0 ? (
            <GlobalText className="text-tertiary-500 diatype-m-regular p-2 text-center">
              No results
            </GlobalText>
          ) : (
            groups.map((group) => (
              <View key={group.key} className="w-full gap-1">
                <GlobalText className="px-1 diatype-sm-bold text-tertiary-500">
                  {group.title}
                </GlobalText>

                {group.items.map((node, idx) => (
                  <MotiView
                    key={`${group.key}-${idx}`}
                    from={childAnim.from}
                    animate={childAnim.animate}
                    transition={{
                      type: "timing",
                      duration: 160,
                      delay: idx * 40,
                    }}
                  >
                    {node}
                  </MotiView>
                ))}
              </View>
            ))
          )}
        </ScrollView>
      </MotiView>
    </AnimatePresence>
  );
};

const ExportComponent = Object.assign(Root, { Body });

export { ExportComponent as SearchMenu };
