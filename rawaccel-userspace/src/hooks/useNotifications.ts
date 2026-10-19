import { useCallback, useState } from 'react';

export interface Notification {
  isOpen: boolean;
  title: string;
  message: string;
}
export type Notify = (notification: Notification) => void;

export function useNotifications() {
  const [notification, notify] = useState<Notification>({ isOpen: false, title: '', message: '' });
  const dismiss = useCallback(() => notify(current => ({ ...current, isOpen: false })), []);
  return { notification, notify, dismiss };
}
