import { InputHTMLAttributes, useState, useEffect, useId, useRef } from "react";

interface NumberInputProps extends Omit<InputHTMLAttributes<HTMLInputElement>, 'onChange' | 'value'> {
  value: number;
  precision?: number;
  onChange: (e: { target: { value: string } }) => void;
}

export function NumberInput({ value = 0, onChange, precision, ...props }: NumberInputProps) {
  const format = (number: number) => {
    if (precision === undefined) return number.toString();
    const padded = number.toFixed(precision);
    // Pad simple stage values without hiding precision in imported/user values.
    return Number(padded) === number ? padded : number.toString();
  };
  const id = useId();
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => {
    const field = input.current;
    const group = field?.closest('.input-group');
    const label = group?.querySelector('label');
    if (field && label && !props['aria-label']) {
      const fields = Array.from(group!.querySelectorAll('input'));
      field.setAttribute('aria-label', `${label.textContent?.trim() ?? ''}${fields.length > 1 ? ` ${fields.indexOf(field) === 0 ? 'X' : 'Y'}` : ''}`);
    }
  });
  const [localValue, setLocalValue] = useState(() => format(value ?? 0));

  useEffect(() => {
    const parsedLocal = parseFloat(localValue);
    const isLocalNaN = isNaN(parsedLocal);
    
    // Only overwrite local string if the actual numeric value changed externally.
    // This allows the user to type empty string "" or "0." without it being immediately overwritten by "0".
    if (
      (!isLocalNaN && parsedLocal !== value) || 
      (isLocalNaN && value !== 0) ||
      (localValue === "" && value !== 0)
    ) {
      setLocalValue(format(value));
    }
  }, [value, localValue]);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    let val = e.target.value;
    
    // Fix the "02" issue: if the previous state was exactly "0" and they typed a number (not a decimal point), 
    // it usually becomes "0X". We strip the leading zero so it becomes "X".
    if (localValue === "0" && val.length === 2 && val.startsWith("0") && val !== "0.") {
      val = val.substring(1);
    }

    setLocalValue(val);
    // Pass a mocked event to the existing onChange handlers which expect e.target.value
    onChange({ target: { value: val } });
  };

  return (
    <input 
      id={props.id ?? id}
      ref={input}
      type="number"
      value={localValue}
      onChange={handleChange}
      onFocus={(e) => {
        // Automatically select the '0' if it's the only thing in the box, 
        // allowing instant typing to replace it.
        if (localValue === "0") {
          e.target.select();
        }
        if (props.onFocus) props.onFocus(e);
      }}
      onBlur={(e) => {
        // Auto-format decimals and empty strings (e.g. ".2" becomes "0.2")
        setLocalValue(format(value));
        if (props.onBlur) props.onBlur(e);
      }}
      {...props}
    />
  );
}
