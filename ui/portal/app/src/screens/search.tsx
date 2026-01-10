import { useRouter } from "expo-router";
import { useTheme } from "~/hooks/useTheme";
import { useFavApplets, useSearchBar } from "@left-curve/store";

import { TextInput, View } from "react-native";
import { SearchMenu } from "~/components/Search/SearchMenu";
import { Button, IconChevronDown, ShadowContainer } from "~/components/foundation";

import { APPLETS } from "~/constants";

export default function SearchScreen() {
  const { theme } = useTheme();
  const { back, replace } = useRouter();
  const { favApplets } = useFavApplets();

  const { searchText, setSearchText, isLoading, isRefetching, searchResult, allNotFavApplets } =
    useSearchBar({
      applets: APPLETS,
      favApplets,
    });

  return (
    <View className="flex-1 flex items-center justify-center bg-surface-primary-rice w-full flex-col gap-8 p-4">
      <View className="flex flex-row justify-center">
        <Button
          variant="link"
          size="icon"
          classNames={{ icons: "text-ink-tertiary-500 rotate-90" }}
          onPress={back}
          rightIcon={<IconChevronDown className="text-ink-tertiary-500" />}
        />
        <View className="flex-1 h-[44px] rounded-md">
          <ShadowContainer style={{ borderRadius: 12, flexGrow: 1, height: 44, width: "100%" }}>
            <TextInput
              value={searchText}
              onChangeText={(t) => setSearchText(t)}
              placeholderTextColor={theme === "dark" ? "#6A5D42" : "#EFDAA4"}
              selectionColor={theme === "dark" ? "#6A5D42" : "#EFDAA4"}
              className="w-full h-full flex justify-center p-2 pl-4 bg-surface-secondary-rice rounded-md text-ink-primary-900"
            />
          </ShadowContainer>
        </View>
      </View>
      <View className="flex-1 w-full">
        <SearchMenu.Body
          isSearching={!!searchText}
          isLoading={isLoading || isRefetching}
          searchResult={searchResult}
          allApplets={allNotFavApplets}
          onSelect={(path: string) => replace(path)}
        />
      </View>
    </View>
  );
}
