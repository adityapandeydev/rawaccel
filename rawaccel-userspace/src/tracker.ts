export interface MouseSpeed {
  speed_x: number;
  speed_y: number;
  speed_whole: number;
}

export function trackerInputs(whole: boolean, speed: MouseSpeed): { axis: 'x' | 'y'; speed: number }[] {
  // Curve equality says nothing about Combined vs By Component processing.
  const inputs = whole ? [{ axis: 'x' as const, speed: speed.speed_whole }]
    : [{ axis: 'x' as const, speed: speed.speed_x }, { axis: 'y' as const, speed: speed.speed_y }];
  return inputs.filter(input => Number.isFinite(input.speed) && input.speed >= 0);
}
