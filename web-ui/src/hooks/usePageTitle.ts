import { useEffect } from 'react';

export function usePageTitle(title: string) {
  useEffect(() => {
    document.title = `${title} · Paqtra`;
    return () => { document.title = 'Paqtra · Zyvor'; };
  }, [title]);
}
