import { useEffect } from 'react';

export function usePageTitle(title: string) {
  useEffect(() => {
    document.title = `${title} | Cilium Vision`;
    return () => { document.title = 'Cilium Vision'; };
  }, [title]);
}
