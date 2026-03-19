import { MutationCache, QueryClient } from "@tanstack/react-query";

const channel = new BroadcastChannel("dango.queries");

export const queryClient = new QueryClient({
  mutationCache: new MutationCache({
    onSettled(_data, _error, _variables, _context, mutation) {
      if (!mutation.meta?.invalidateKeys) return;
      channel.postMessage({ type: "invalidate", keys: mutation.meta.invalidateKeys });
    },
  }),
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: false,
      retry: 0,
    },
  },
});

channel.onmessage = ({ data: event }) => {
  if (event.type === "invalidate") {
    for (const key of event.keys) {
      queryClient.invalidateQueries({ queryKey: key });
    }
  }
};
