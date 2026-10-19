import React, { useEffect, useId, useRef } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { AlertCircle } from "lucide-react";

interface CustomModalProps {
  isOpen: boolean;
  title: string;
  message: string;
  onClose: () => void;
  onConfirm?: () => void;
  confirmLabel?: string;
}

export function CustomModal({ isOpen, title, message, onClose, onConfirm, confirmLabel = 'Confirm' }: CustomModalProps) {
  const content = useRef<HTMLDivElement>(null);
  const titleId = useId();
  const messageId = useId();
  const closeRef = useRef(onClose);
  closeRef.current = onClose;
  useEffect(() => {
    if (!isOpen) return;
    const previous = document.activeElement as HTMLElement | null;
    content.current?.querySelector<HTMLElement>('button')?.focus();
    const key = (event: KeyboardEvent) => {
      if (event.key === 'Escape') { event.preventDefault(); closeRef.current(); }
      if (event.key !== 'Tab') return;
      const buttons = Array.from(content.current?.querySelectorAll<HTMLElement>('button') ?? []);
      const index = buttons.indexOf(document.activeElement as HTMLElement);
      event.preventDefault();
      buttons[(index + (event.shiftKey ? -1 : 1) + buttons.length) % buttons.length]?.focus();
    };
    document.addEventListener('keydown', key);
    return () => { document.removeEventListener('keydown', key); previous?.focus(); };
  }, [isOpen]);
  return (
    <AnimatePresence>
      {isOpen && (
        <div className="modal-overlay" onClick={onClose}>
          <motion.div
            ref={content} role="dialog" aria-modal="true" aria-labelledby={titleId} aria-describedby={messageId}
            className="modal-content"
            initial={{ opacity: 0, scale: 0.9, y: 20 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.9, y: 20 }}
            transition={{ type: "spring", damping: 25, stiffness: 300 }}
            onClick={(e) => e.stopPropagation()} // Prevent closing when clicking inside
          >
            <div className="modal-header">
              <AlertCircle size={24} color="var(--color-accent)" />
              <span id={titleId}>{title}</span>
            </div>
            <div className="modal-body" id={messageId}>
              {message.split('\n').map((line, i) => (
                <React.Fragment key={i}>
                  {line}
                  {i < message.split('\n').length - 1 && <br />}
                </React.Fragment>
              ))}
            </div>
            <div className="modal-footer">
              <button onClick={onClose}>{onConfirm ? 'Cancel' : 'OK'}</button>
              {onConfirm && <button onClick={() => { onConfirm(); onClose(); }}>{confirmLabel}</button>}
            </div>
          </motion.div>
        </div>
      )}
    </AnimatePresence>
  );
}
