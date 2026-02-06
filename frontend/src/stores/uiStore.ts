import { create } from 'zustand';

interface UIState {
  sidebarCollapsed: boolean;
  selectedHop: number | null;
  detailPanelOpen: boolean;

  toggleSidebar: () => void;
  selectHop: (hopNumber: number | null) => void;
  openDetailPanel: () => void;
  closeDetailPanel: () => void;
}

export const useUIStore = create<UIState>()((set) => ({
  sidebarCollapsed: false,
  selectedHop: null,
  detailPanelOpen: false,

  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
  selectHop: (hopNumber) => set({ selectedHop: hopNumber, detailPanelOpen: hopNumber !== null }),
  openDetailPanel: () => set({ detailPanelOpen: true }),
  closeDetailPanel: () => set({ detailPanelOpen: false }),
}));
