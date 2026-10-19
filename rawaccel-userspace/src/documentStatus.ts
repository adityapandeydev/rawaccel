export function documentStatus(current: string, saved: string | null, origin: 'loaded' | 'saved', applied: string | null) {
  const edited = current !== saved;
  return {
    edited,
    disk: edited ? 'Unsaved changes' : origin === 'saved' ? 'Saved to disk' : 'Loaded from disk',
    driver: current === applied ? 'Applied this session' : applied ? 'Changed since last Apply' : 'Driver state not verified',
  };
}
