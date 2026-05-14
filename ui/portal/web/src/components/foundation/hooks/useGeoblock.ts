import { useQuery } from "@tanstack/react-query";

const RESTRICTED_COUNTRIES = new Set<string>(["US"]);

const fetchCountry = async (): Promise<string | null> => {
  const res = await fetch("/cdn-cgi/trace");
  if (!res.ok) return null;
  const text = await res.text();
  const match = text.match(/^loc=([A-Z]{2})$/m);
  return match?.[1] ?? null;
};

export function useGeoblock(): boolean {
  const { data: country } = useQuery({
    queryKey: ["geo"] as const,
    queryFn: fetchCountry,
    staleTime: Number.POSITIVE_INFINITY,
    retry: false,
  });

  return country ? RESTRICTED_COUNTRIES.has(country) : false;
}
