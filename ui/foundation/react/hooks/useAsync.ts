// forked from: https://github.com/streamich/react-use/blob/master/src/useAsync.ts
import { type DependencyList, useEffect } from "react";
import { useAsyncFn, type FunctionReturningPromise } from "./useAsyncFn";

export function useAsync<T extends FunctionReturningPromise>(fn: T, deps: DependencyList = []) {
  const [state, callback] = useAsyncFn(fn, deps, {
    loading: true,
  });

  useEffect(() => {
    callback();
  }, [callback]);

  return state;
}
