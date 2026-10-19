import { useEffect, useState } from 'react';

// Preference behavior is intentionally unchanged here. Persistence and system
// theme support belong to the later product/design phases.
export function usePreferences() {
  const [theme, setTheme] = useState<'dark' | 'light' | 'system'>('dark');
  const [language, setLanguage] = useState('en-US');
  const [showToastNotifications, setShowToastNotifications] = useState(true);
  const [showConfirmModals, setShowConfirmModals] = useState(true);
  useEffect(() => {
    if (theme === 'light') document.body.setAttribute('data-theme', 'light');
    else document.body.removeAttribute('data-theme');
  }, [theme]);
  return { theme, setTheme, language, setLanguage, showToastNotifications,
    setShowToastNotifications, showConfirmModals, setShowConfirmModals };
}
