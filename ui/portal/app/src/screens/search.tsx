import { useFavApplets, useSearchBar } from "@left-curve/store";
import { useRouter } from "expo-router";
import { TextInput, View } from "react-native";
import { Button, IconChevronDown } from "~/components/foundation";
import { SearchMenu } from "~/components/Search/SearchMenu";
import { useTheme } from "~/hooks/useTheme";
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
          classNames={{ icons: "text-tertiary-500 rotate-90" }}
          onPress={back}
          rightIcon={<IconChevronDown className="text-tertiary-500" />}
        />
        <TextInput
          value={searchText}
          onChangeText={(t) => setSearchText(t)}
          placeholderTextColor={theme === "dark" ? "#6A5D42" : "#EFDAA4"}
          selectionColor={theme === "dark" ? "#6A5D42" : "#EFDAA4"}
          className="flex-1 h-[44px] flex justify-center p-2  pl-4 shadow shadow-btn-shadow-gradient bg-surface-secondary-rice rounded-md text-primary-900"
        />
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
