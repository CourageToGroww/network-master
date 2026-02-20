import { useMutation, useQuery } from '@tanstack/react-query';
import { api, queryClient } from '../client';
import type { UpdateInfo } from '../../types';

export function useUpdateInfo() {
  return useQuery({
    queryKey: ['update-info'],
    queryFn: () => api.get<UpdateInfo>('/update/info'),
    retry: false,
    staleTime: 10_000,
  });
}

export function useUploadBinary() {
  return useMutation({
    mutationFn: async ({ file, version }: { file: File; version: string }) => {
      const formData = new FormData();
      formData.append('binary', file);
      formData.append('version', version);

      const res = await fetch('/api/v1/update/upload', {
        method: 'POST',
        body: formData,
      });

      if (!res.ok) throw new Error(`Upload failed: ${res.status}`);
      return res.json() as Promise<UpdateInfo>;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['update-info'] });
    },
  });
}

export function useTriggerUpdate() {
  return useMutation({
    mutationFn: async (agentId: string) => {
      const res = await fetch(`/api/v1/agents/${agentId}/update`, {
        method: 'POST',
      });
      if (!res.ok) throw new Error(`Trigger update failed: ${res.status}`);
    },
  });
}

export function useTriggerUpdateAll() {
  return useMutation({
    mutationFn: async () => {
      const res = await fetch('/api/v1/update/push-all', {
        method: 'POST',
      });
      if (!res.ok) throw new Error(`Push all failed: ${res.status}`);
      return res.json() as Promise<{ pushed: number; total_online: number; version: string }>;
    },
  });
}
