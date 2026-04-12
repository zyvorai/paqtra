import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface NamespaceStore {
  selectedNamespace: string; // '' means all namespaces
  setNamespace: (ns: string) => void;
  clearNamespace: () => void;
}

export const useNamespaceStore = create<NamespaceStore>()(
  persist(
    (set) => ({
      selectedNamespace: '',
      setNamespace: (ns) => set({ selectedNamespace: ns }),
      clearNamespace: () => set({ selectedNamespace: '' }),
    }),
    { name: 'cilium-vision-namespace' }
  )
);
