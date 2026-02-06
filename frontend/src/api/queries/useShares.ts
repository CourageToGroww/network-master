import { useQuery } from '@tanstack/react-query';
import { api } from '../client';
import type { ShareToken } from '../../types';

export function useShareTokens(targetId: string) {
  return useQuery({
    queryKey: ['share-tokens', targetId],
    queryFn: () => api.get<ShareToken[]>(`/targets/${targetId}/shares`),
    enabled: !!targetId,
  });
}
