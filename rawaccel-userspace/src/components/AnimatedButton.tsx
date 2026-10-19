import React, { useEffect, useRef, useState } from "react";

interface AnimatedButtonProps {
  onClick: () => Promise<void | boolean> | void | boolean;
  defaultText: React.ReactNode;
  successText: React.ReactNode;
  className?: string;
  disabled?: boolean;
  title?: string;
}

export function AnimatedButton({ onClick, defaultText, successText, className = "btn btn-primary", disabled = false, title }: AnimatedButtonProps) {
  const [isSuccess, setIsSuccess] = useState(false);
  const [isPending, setIsPending] = useState(false);
  const busy = useRef(false);
  const mounted = useRef(true);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      if (timer.current !== null) clearTimeout(timer.current);
    };
  }, []);

  const handleClick = async () => {
    if (busy.current || disabled) return;
    busy.current = true;
    setIsPending(true);
    try {
      const succeeded = await onClick();
      // A completed operation may have a partial outcome already shown by its
      // caller. Do not flash full success for applied-but-unsaved settings.
      if (!mounted.current || succeeded === false) { busy.current = false; return; }
      setIsSuccess(true);
      timer.current = setTimeout(() => {
        busy.current = false;
        setIsSuccess(false);
        timer.current = null;
      }, 2000);
    } catch (e) {
      busy.current = false;
      // Error handled by parent
    } finally {
      if (mounted.current) setIsPending(false);
    }
  };

  return (
    <button
      className={`${className} ${isSuccess ? "btn-success-animated" : ""}`}
      onClick={handleClick}
      disabled={isPending || isSuccess || disabled}
      aria-busy={isPending}
      title={title}
    >
      {isSuccess ? successText : defaultText}
    </button>
  );
}
