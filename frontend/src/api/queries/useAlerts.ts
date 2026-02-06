import { useQuery } from '@tanstack/react-query';
import { api } from '../client';
import type { AlertEvent, AlertRule, DashboardSummary } from '../../types';

export function useAlertRules() {
  return useQuery({
    queryKey: ['alert-rules'],
    queryFn: () => api.get<AlertRule[]>('/alert-rules'),
  });
}

export function useAlertEvents(limit = 100) {
  return useQuery({
    queryKey: ['alert-events', limit],
    queryFn: () => api.get<AlertEvent[]>(`/alert-events?limit=${limit}`),
  });
}

export function useDashboardSummary() {
  return useQuery({
    queryKey: ['dashboard-summary'],
    queryFn: () => api.get<DashboardSummary>('/dashboard/summary'),
    refetchInterval: 10_000,
  });
}
