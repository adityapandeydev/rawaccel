import { useEffect, useId, useRef } from "react";
interface Option { value: string | number; label: string; }
interface CustomSelectProps {
  value: string | number; options: Option[];
  onChange: (value: any) => void; width?: string; label?: string;
}
// A native select provides keyboard navigation, type-ahead, Escape, platform
// accessibility and a popup that is never clipped by scrolling inspectors.
export function CustomSelect({ value, options, onChange, width = "120px", label }: CustomSelectProps) {
  const id = useId();
  const input = useRef<HTMLSelectElement>(null);
  useEffect(() => {
    if (label) return;
    const text = input.current?.closest('.input-group')?.querySelector('label')?.textContent;
    if (text) input.current?.setAttribute('aria-label', text);
  });
  const selected = options.findIndex(option => option.value === value);
  return <select ref={input} id={id} className="input-field selectable custom-select" style={{ width }}
    aria-label={label ?? options[selected < 0 ? 0 : selected]?.label}
    value={selected < 0 ? 0 : selected}
    onChange={event => onChange(options[Number(event.target.value)].value)}>
    {options.map((option, index) => <option key={option.value} value={index}>{option.label}</option>)}
  </select>;
}
