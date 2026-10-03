import { QueryClient } from "@tanstack/vue-query";

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 5 * 60 * 1000,
      gcTime: 30 * 60 * 1000,
      // Tauri commands work independently of the browser's network status.
      networkMode: "always",
      retry: false,
      refetchOnWindowFocus: false,
    },
    mutations: { networkMode: "always" },
  },
});

export const queryKeys = {
  companies: ["companies"] as const,
  company: (companyId: string) => ["company", companyId] as const,
  snapshot: (companyId: string) => ["company", companyId, "snapshot"] as const,
  parameters: (companyId: string) =>
    ["company", companyId, "parameters"] as const,
  monthlyLedger: (companyId: string, year: number) =>
    ["company", companyId, "monthly-ledger", year] as const,
  cashBook: (companyId: string, year: number, channel: string) =>
    ["company", companyId, "cash-book", year, channel] as const,
};
