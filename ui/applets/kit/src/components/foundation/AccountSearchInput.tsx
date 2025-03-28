import { capitalize } from "@left-curve/dango/utils";
import { usePublicClient } from "@left-curve/store";
import { forwardRef, useDOMRef } from "../../utils";
import { SearchInput, type SearchInputProps } from "./SearchInput";

type Props = Omit<SearchInputProps, "getOptionsFn">;

export const AccountSearchInput = forwardRef<"input", Props>((props, ref) => {
  const client = usePublicClient();

  const searchRef = useDOMRef(ref);

  async function getOptionsFn(inputValue: string) {
    if (inputValue.includes("0x") || !inputValue) return [];
    const { accounts } = await client.getUser({ username: inputValue });
    return Object.entries(accounts).map(([address, account]) => ({
      key: `${capitalize(Object.keys(account.params).at(0) as string)} Account #${account.index}`,
      value: address,
    }));
  }

  return <SearchInput {...props} type="text" ref={searchRef} getOptionsFn={getOptionsFn} />;
});
