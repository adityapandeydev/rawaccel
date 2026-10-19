import { useEffect, useState } from 'react';
import palette from '../palette.json';
import { parseTheme, resolveTheme, THEME_KEY, type ThemePreference } from '../theme';

export function usePreferences() {
  const [theme, setTheme] = useState<ThemePreference>(() => {
    try { return parseTheme(localStorage.getItem(THEME_KEY)); } catch { return 'system'; }
  });
  const [language, setLanguage] = useState('en-US');
  const [showToastNotifications, setShowToastNotifications] = useState(true);
  const [showConfirmModals, setShowConfirmModals] = useState(true);
  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: dark)');
    const apply = () => {
      const resolved = resolveTheme(theme, media.matches);
      document.body.dataset.theme = resolved;
      for (const [role, color] of Object.entries(palette[resolved])) document.body.style.setProperty(`--color-${role}`, color);
    };
    apply();
    media.addEventListener('change', apply);
    try { localStorage.setItem(THEME_KEY, theme); } catch { /* Session-only if storage is unavailable. */ }
    return () => media.removeEventListener('change', apply);
  }, [theme]);
  return { theme, setTheme, language, setLanguage, showToastNotifications,
    setShowToastNotifications, showConfirmModals, setShowConfirmModals };
}
