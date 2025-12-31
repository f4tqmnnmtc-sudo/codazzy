"use client";

import { useEffect, useCallback, type ReactNode, type ComponentPropsWithoutRef } from "react";
import { createPortal } from "react-dom";
import { cn } from "@/lib/utils";
import { IconButton } from "./primitives";

interface ModalProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  description?: string;
  icon?: ReactNode;
  children: ReactNode;
  footer?: ReactNode;
  width?: "sm" | "md" | "lg" | "xl";
  closeOnOverlayClick?: boolean;
  closeOnEscape?: boolean;
}

const widthClasses = {
  sm: "w-[360px]",
  md: "w-[450px]",
  lg: "w-[600px]",
  xl: "w-[800px]",
};

export function Modal({
  open,
  onClose,
  title,
  description,
  icon,
  children,
  footer,
  width = "md",
  closeOnOverlayClick = true,
  closeOnEscape = true,
}: ModalProps) {
  const handleEscape = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Escape" && closeOnEscape) onClose();
    },
    [onClose, closeOnEscape]
  );

  useEffect(() => {
    if (!open) return;

    document.addEventListener("keydown", handleEscape);
    document.body.style.overflow = "hidden";

    return () => {
      document.removeEventListener("keydown", handleEscape);
      document.body.style.overflow = "";
    };
  }, [open, handleEscape]);

  if (!open) return null;

  const content = (
    <>
      {/* Overlay */}
      <div
        className="fixed inset-0 bg-black/60 z-50 animate-in fade-in duration-200"
        onClick={closeOnOverlayClick ? onClose : undefined}
        aria-hidden="true"
      />

      {/* Dialog */}
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby={title ? "modal-title" : undefined}
        aria-describedby={description ? "modal-description" : undefined}
        className={cn(
          "fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50",
          "bg-[#131a26] border border-[#2a3548] rounded-xl shadow-2xl",
          "animate-in zoom-in-95 fade-in duration-200",
          widthClasses[width]
        )}
      >
        {/* Header */}
        {(title || icon) && (
          <div className="flex items-center justify-between px-5 py-4 border-b border-[#2a3548]">
            <div className="flex items-center gap-3">
              {icon && <span className="text-emerald-400">{icon}</span>}
              {title && (
                <div>
                  <h3 id="modal-title" className="text-[15px] font-semibold text-white">
                    {title}
                  </h3>
                  {description && (
                    <p id="modal-description" className="text-[12px] text-[#8b95a5] mt-0.5">
                      {description}
                    </p>
                  )}
                </div>
              )}
            </div>
            <IconButton label="Cerrar" variant="ghost" size="md" onClick={onClose}>
              <svg className="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M18 6L6 18M6 6l12 12" />
              </svg>
            </IconButton>
          </div>
        )}

        {/* Content */}
        <div className="p-5">{children}</div>

        {/* Footer */}
        {footer && (
          <div className="flex gap-3 px-5 py-4 border-t border-[#2a3548]">{footer}</div>
        )}
      </div>
    </>
  );

  // Portal to body
  if (typeof document === "undefined") return null;
  return createPortal(content, document.body);
}


interface ConfirmDialogProps {
  open: boolean;
  onClose: () => void;
  onConfirm: () => void;
  title: string;
  description?: string;
  confirmText?: string;
  cancelText?: string;
  variant?: "danger" | "warning" | "default";
  loading?: boolean;
}

export function ConfirmDialog({
  open,
  onClose,
  onConfirm,
  title,
  description,
  confirmText = "Confirmar",
  cancelText = "Cancelar",
  variant = "default",
  loading,
}: ConfirmDialogProps) {
  const variantStyles = {
    danger: "bg-red-500 hover:bg-red-600 text-white",
    warning: "bg-amber-500 hover:bg-amber-600 text-white",
    default: "bg-emerald-500 hover:bg-emerald-600 text-[#0a0e17]",
  };

  const icons = {
    danger: (
      <svg className="w-6 h-6 text-red-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <circle cx="12" cy="12" r="10" />
        <path d="M15 9l-6 6M9 9l6 6" />
      </svg>
    ),
    warning: (
      <svg className="w-6 h-6 text-amber-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
        <line x1="12" y1="9" x2="12" y2="13" />
        <line x1="12" y1="17" x2="12.01" y2="17" />
      </svg>
    ),
    default: (
      <svg className="w-6 h-6 text-emerald-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <circle cx="12" cy="12" r="10" />
        <path d="M12 16v-4M12 8h.01" />
      </svg>
    ),
  };

  return (
    <Modal open={open} onClose={onClose} width="sm">
      <div className="text-center">
        <div className="mx-auto w-12 h-12 rounded-full bg-[#0a0e17] flex items-center justify-center mb-4">
          {icons[variant]}
        </div>
        <h3 className="text-[16px] font-semibold text-white mb-2">{title}</h3>
        {description && <p className="text-[13px] text-[#8b95a5] mb-6">{description}</p>}
        <div className="flex gap-3">
          <button
            onClick={onClose}
            disabled={loading}
            className="flex-1 px-4 py-2 bg-[#2a3548] hover:bg-[#3a4558] text-white rounded-lg text-[13px] font-medium transition-colors disabled:opacity-50"
          >
            {cancelText}
          </button>
          <button
            onClick={onConfirm}
            disabled={loading}
            className={cn(
              "flex-1 px-4 py-2 rounded-lg text-[13px] font-medium transition-colors disabled:opacity-50",
              variantStyles[variant]
            )}
          >
            {loading ? "..." : confirmText}
          </button>
        </div>
      </div>
    </Modal>
  );
}

