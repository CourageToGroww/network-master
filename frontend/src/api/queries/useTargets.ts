import { useQuery } from '@tanstack/react-query';
import { api } from '../client';
import type { Target } from '../../types';

export function useTargets(agentId: string) {
  return useQuery({
    queryKey: ['targets', agentId],
    queryFn: () => api.get<Target[]>(`/agents/${agentId}/targets`),
    enabled: !!agentId,
  });
}

export function useTarget(id: string) {
  return useQuery({
    queryKey: ['target', id],
    queryFn: () => api.get<Target>(`/targets/${id}`),
    enabled: !!id,
  });
}
