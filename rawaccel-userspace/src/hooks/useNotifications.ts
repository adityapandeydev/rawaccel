import { useCallback, useEffect, useRef, useState } from 'react';
import { notificationChannel } from '../preferences';

export interface Notification {
  isOpen: boolean;
  title: string;
  message: string;
  kind?: 'info' | 'error';
}
export type Notify = (notification: Notification) => void;

export function useNotifications(showToastNotifications: boolean) {
  const [notification, setNotification] = useState<Notification>({ isOpen: false, title: '', message: '' });
  const [toast, setToast] = useState<Notification>({ isOpen: false, title: '', message: '' });
  const enabled = useRef(showToastNotifications);
  enabled.current = showToastNotifications;
  // A stable callback keeps preference changes from restarting document loads.
  const notify = useCallback((message: Notification) => {
    const channel = notificationChannel(message.kind, enabled.current);
    if (channel === 'toast') setToast(message);
    if (channel === 'dialog') setNotification(message);
  }, []);
  const dismiss = useCallback(() => setNotification(current => ({ ...current, isOpen: false })), []);
  const dismissToast = useCallback(() => setToast(current => ({ ...current, isOpen: false })), []);
  useEffect(() => {
    if (!showToastNotifications) dismissToast();
  }, [showToastNotifications, dismissToast]);
  useEffect(() => {
    if (!toast.isOpen) return;
    const timer = setTimeout(dismissToast, 6000);
    return () => clearTimeout(timer);
  }, [toast, dismissToast]);
  return { notification, toast, notify, dismiss, dismissToast };
}
