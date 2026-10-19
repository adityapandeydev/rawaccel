import { useEffect, useState } from 'react';

export function useNavigation() {
  const [wide, setWide] = useState(() => window.matchMedia('(min-width: 1280px)').matches);
  const [collapsed, setCollapsed] = useState(() => { try { return localStorage.getItem('rawaccel.sidebar') === 'collapsed'; } catch { return false; } });
  const [drawer, setDrawer] = useState(false);
  useEffect(() => {
    const media = window.matchMedia('(min-width: 1280px)');
    const update = () => { setWide(media.matches); setDrawer(false); };
    media.addEventListener('change', update);
    return () => media.removeEventListener('change', update);
  }, []);
  const toggle = () => {
    if (!wide) { setDrawer(value => !value); return; }
    setCollapsed(value => {
      try { localStorage.setItem('rawaccel.sidebar', value ? 'open' : 'collapsed'); } catch { /* Session-only. */ }
      return !value;
    });
  };
  return { wide, open: wide ? !collapsed : drawer, toggle, closeDrawer: () => setDrawer(false) };
}
