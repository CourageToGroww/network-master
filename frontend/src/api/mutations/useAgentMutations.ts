import { useMutation } from '@tanstack/react-query';
import { api, queryClient } from '../client';
import type { AgentRegistration } from '../../types';

export function useRegisterAgent() {
  return useMutation({
    mutationFn: (name: string) =>
      api.post<AgentRegistration>('/agents', { name }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['agents'] });
    },
  });
}

export function useDeleteAgent() {
  return useMutation({
    mutationFn: (id: string) => api.delete(`/agents/${id}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['agents'] });
    },
  });
}
