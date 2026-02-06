import { useQuery } from '@tanstack/react-query';
import { api } from '../client';
import type { TraceProfile } from '../../types';

export function useTraceProfiles() {
  return useQuery({
    queryKey: ['trace-profiles'],
    queryFn: () => api.get<TraceProfile[]>('/trace-profiles'),
  });
}

export function useTraceProfile(id: string) {
  return useQuery({
    queryKey: ['trace-profiles', id],
    queryFn: () => api.get<TraceProfile>(`/trace-profiles/${id}`),
    enabled: !!id,
  });
}
