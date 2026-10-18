import React, { useState } from "react";

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

  const handleClick = async () => {
    if (isSuccess || disabled) return;
    try {
      const succeeded = await onClick();
      // A completed operation may have a partial outcome already shown by its
      // caller. Do not flash full success for applied-but-unsaved settings.
      if (succeeded === false) return;
      setIsSuccess(true);
      setTimeout(() => setIsSuccess(false), 2000);
    } catch (e) {
      // Error handled by parent
    }
  };

  return (
    <button
      className={`${className} ${isSuccess ? "btn-success-animated" : ""}`}
      onClick={handleClick}
      disabled={isSuccess || disabled}
      title={title}
    >
      {isSuccess ? successText : defaultText}
    </button>
  );
}
