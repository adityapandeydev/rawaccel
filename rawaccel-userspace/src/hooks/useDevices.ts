import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Device } from '../types';

export function useDevices() {
  const [devices, setDevices] = useState<Device[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const pending = useRef<Promise<void> | null>(null);
  const mounted = useRef(true);
  const refresh = useCallback((): Promise<void> => {
    if (pending.current) return pending.current;
    setLoading(true);
    setError(null);
    const request = invoke<Device[]>('get_devices').then(result => {
      if (mounted.current) setDevices(result);
    }).catch(reason => {
      if (mounted.current) setError(String(reason));
    }).finally(() => {
      pending.current = null;
      if (mounted.current) setLoading(false);
    });
    pending.current = request;
    return request;
  }, []);
  useEffect(() => {
    mounted.current = true;
    void refresh();
    return () => { mounted.current = false; };
  }, [refresh]);
  return { devices, loading, error, refresh };
}
