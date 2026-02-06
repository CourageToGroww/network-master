import { useMutation } from '@tanstack/react-query';
import { api, queryClient } from '../client';
import type { CreateTarget, Target } from '../../types';

export function useCreateTarget(agentId: string) {
  return useMutation({
    mutationFn: (input: CreateTarget) =>
      api.post<Target>(`/agents/${agentId}/targets`, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['targets', agentId] });
    },
  });
}

export function useDeleteTarget() {
  return useMutation({
    mutationFn: (id: string) => api.delete(`/targets/${id}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['targets'] });
    },
  });
}
