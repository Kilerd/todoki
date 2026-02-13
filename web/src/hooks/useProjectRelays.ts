import { useCallback, useEffect, useState } from "react";
import * as api from "../api/relays";
import type { RelayInfo } from "../api/types";

export function useProjectRelays(projectId: string | undefined) {
  const [relays, setRelays] = useState<RelayInfo[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const refresh = useCallback(async () => {
    if (!projectId) {
      setRelays([]);
      return;
    }

    setIsLoading(true);
    setError(null);
    try {
      const { data } = await api.fetchProjectRelays({ project_id: projectId });
      setRelays(data as RelayInfo[]);
    } catch (err) {
      setError(err instanceof Error ? err : new Error("Failed to fetch relays"));
      setRelays([]);
    } finally {
      setIsLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { relays, isLoading, error, refresh };
}
