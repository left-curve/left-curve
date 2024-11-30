import { usePublicClient } from "@left-curve/react";
import { forwardRef, useDOMRef } from "../../utils";
import Address from "../atoms/Address";
import { SearchInput, type SearchInputProps } from "./SearchInput";

type Props = Omit<SearchInputProps, "getOptionsFn">;

export const AccountSearchInput = forwardRef<"input", Props>((props, ref) => {
  const client = usePublicClient();

  const searchRef = useDOMRef(ref);

  async function getOptionsFn(inputValue: string) {
    if (inputValue.includes("0x") || !inputValue) return [];
    const { accounts } = await client.getUser({ username: inputValue });
    return Object.keys(accounts);
  }

  return (
    <SearchInput
      {...props}
      ref={searchRef}
      getOptionsFn={getOptionsFn}
      optComponent={<Address />}
    />
  );
});
