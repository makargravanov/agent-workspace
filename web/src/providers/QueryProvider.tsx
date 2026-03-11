import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { type ReactNode, useState } from 'react';

interface QueryProviderProps {
  children: ReactNode;
}

export function QueryProvider({ children }: QueryProviderProps) {
  // Create one client per component tree mount (stable across re-renders,
  // isolated from tests or concurrent React roots if needed).
  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            // Avoid redundant refetches when the user switches tabs quickly.
            staleTime: 30_000,
            // Fail fast: surface errors to error boundaries, do not silently retry forever.
            retry: 1,
          },
        },
      }),
  );

  return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
}
