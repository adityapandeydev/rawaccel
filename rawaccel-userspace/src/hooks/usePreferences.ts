import { useEffect, useState } from 'react';
import palette from '../palette.json';
import { parseTheme, resolveTheme, THEME_KEY, type ThemePreference } from '../theme';
import { parsePreferences, PREFERENCES_KEY } from '../preferences';

export function usePreferences() {
  const [theme, setTheme] = useState<ThemePreference>(() => {
    try { return parseTheme(localStorage.getItem(THEME_KEY)); } catch { return 'system'; }
  });
  const [resolvedTheme, setResolvedTheme] = useState<'dark' | 'light'>(() => resolveTheme(theme, window.matchMedia('(prefers-color-scheme: dark)').matches));
  const [preferences, setPreferences] = useState(() => {
    try { return parsePreferences(localStorage.getItem(PREFERENCES_KEY)); } catch { return parsePreferences(null); }
  });
  const setShowToastNotifications = (value: boolean) => setPreferences(previous => ({ ...previous, showToastNotifications: value }));
  const setShowConfirmModals = (value: boolean) => setPreferences(previous => ({ ...previous, showConfirmModals: value }));
  useEffect(() => {
    try { localStorage.setItem(PREFERENCES_KEY, JSON.stringify(preferences)); } catch { /* Session-only if storage is unavailable. */ }
  }, [preferences]);
  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: dark)');
    const apply = () => {
      const resolved = resolveTheme(theme, media.matches);
      document.body.dataset.theme = resolved;
      setResolvedTheme(resolved);
      for (const [role, color] of Object.entries(palette[resolved])) document.body.style.setProperty(`--color-${role}`, color);
    };
    apply();
    media.addEventListener('change', apply);
    try { localStorage.setItem(THEME_KEY, theme); } catch { /* Session-only if storage is unavailable. */ }
    return () => media.removeEventListener('change', apply);
  }, [theme]);
  return { theme, setTheme, resolvedTheme, ...preferences,
    setShowToastNotifications, setShowConfirmModals };
}
