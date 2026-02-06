import { create } from 'zustand';

interface AgentState {
  selectedAgentId: string | null;
  onlineStatus: Map<string, boolean>;

  selectAgent: (id: string | null) => void;
  setAgentOnline: (agentId: string, isOnline: boolean) => void;
}

export const useAgentStore = create<AgentState>()((set) => ({
  selectedAgentId: null,
  onlineStatus: new Map(),

  selectAgent: (id) => set({ selectedAgentId: id }),

  setAgentOnline: (agentId, isOnline) =>
    set((state) => {
      const next = new Map(state.onlineStatus);
      next.set(agentId, isOnline);
      return { onlineStatus: next };
    }),
}));
