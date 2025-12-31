"use client";

import { forwardRef, type ComponentPropsWithoutRef, type ReactNode } from "react";
import { cn } from "@/lib/utils";
import {
  buttonVariants,
  inputVariants,
  cardVariants,
  badgeVariants,
  labelVariants,
  statusDotVariants,
  progressBarVariants,
  iconButtonVariants,
  type ButtonVariants,
  type InputVariants,
  type CardVariants,
  type BadgeVariants,
  type LabelVariants,
  type StatusDotVariants,
  type ProgressBarVariants,
  type IconButtonVariants,
} from "@/lib/variants";

interface ButtonProps extends ComponentPropsWithoutRef<"button">, ButtonVariants {
  loading?: boolean;
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, loading, disabled, children, ...props }, ref) => (
    <button
      ref={ref}
      className={cn(buttonVariants({ variant, size }), className)}
      disabled={disabled || loading}
      {...props}
    >
      {loading && (
        <svg className="w-4 h-4 animate-spin" viewBox="0 0 24 24" fill="none">
          <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
          <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
        </svg>
      )}
      {children}
    </button>
  )
);
Button.displayName = "Button";


interface IconButtonProps extends ComponentPropsWithoutRef<"button">, IconButtonVariants {
  label: string;
}

export const IconButton = forwardRef<HTMLButtonElement, IconButtonProps>(
  ({ className, variant, size, label, children, ...props }, ref) => (
    <button
      ref={ref}
      aria-label={label}
      className={cn(iconButtonVariants({ variant, size }), className)}
      {...props}
    >
      {children}
    </button>
  )
);
IconButton.displayName = "IconButton";


interface InputProps extends ComponentPropsWithoutRef<"input">, InputVariants {}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ className, variant, size, ...props }, ref) => (
    <input
      ref={ref}
      className={cn(inputVariants({ variant, size }), className)}
      {...props}
    />
  )
);
Input.displayName = "Input";


interface TextareaProps extends ComponentPropsWithoutRef<"textarea">, InputVariants {}

export const Textarea = forwardRef<HTMLTextAreaElement, TextareaProps>(
  ({ className, variant, size, ...props }, ref) => (
    <textarea
      ref={ref}
      className={cn(inputVariants({ variant, size }), "resize-none", className)}
      {...props}
    />
  )
);
Textarea.displayName = "Textarea";


interface SelectProps extends ComponentPropsWithoutRef<"select">, InputVariants {}

export const Select = forwardRef<HTMLSelectElement, SelectProps>(
  ({ className, variant, size, children, ...props }, ref) => (
    <select
      ref={ref}
      className={cn(inputVariants({ variant, size }), "cursor-pointer", className)}
      {...props}
    >
      {children}
    </select>
  )
);
Select.displayName = "Select";


interface CardProps extends ComponentPropsWithoutRef<"div">, CardVariants {}

export const Card = forwardRef<HTMLDivElement, CardProps>(
  ({ className, variant, padding, ...props }, ref) => (
    <div
      ref={ref}
      className={cn(cardVariants({ variant, padding }), className)}
      {...props}
    />
  )
);
Card.displayName = "Card";


interface BadgeProps extends ComponentPropsWithoutRef<"span">, BadgeVariants {}

export const Badge = forwardRef<HTMLSpanElement, BadgeProps>(
  ({ className, variant, size, ...props }, ref) => (
    <span
      ref={ref}
      className={cn(badgeVariants({ variant, size }), className)}
      {...props}
    />
  )
);
Badge.displayName = "Badge";


interface LabelProps extends ComponentPropsWithoutRef<"label">, LabelVariants {}

export const Label = forwardRef<HTMLLabelElement, LabelProps>(
  ({ className, variant, size, ...props }, ref) => (
    <label
      ref={ref}
      className={cn(labelVariants({ variant, size }), "mb-1.5", className)}
      {...props}
    />
  )
);
Label.displayName = "Label";


interface StatusDotProps extends ComponentPropsWithoutRef<"span">, StatusDotVariants {
  pulse?: boolean;
}

export function StatusDot({ status, size, pulse, className }: StatusDotProps) {
  return (
    <span className="relative flex">
      {pulse && status === "online" && (
        <span
          className={cn(
            "absolute inline-flex h-full w-full rounded-full opacity-75 animate-ping",
            statusDotVariants({ status, size })
          )}
        />
      )}
      <span className={cn(statusDotVariants({ status, size }), className)} />
    </span>
  );
}


interface ProgressBarProps extends ProgressBarVariants {
  value: number;
  max?: number;
  showLabel?: boolean;
  className?: string;
}

export function ProgressBar({ value, max = 100, variant, showLabel, className }: ProgressBarProps) {
  const percent = Math.min(100, Math.max(0, (value / max) * 100));
  const barVariant = percent > 80 ? "danger" : variant;

  return (
    <div className={cn("flex items-center gap-3", className)}>
      <div className="flex-1 h-1.5 bg-[#2a3548] rounded-full overflow-hidden">
        <div
          className={progressBarVariants({ variant: barVariant })}
          style={{ width: `${percent}%` }}
        />
      </div>
      {showLabel && (
        <span className="text-[12px] font-medium text-white w-10 text-right">
          {Math.round(percent)}%
        </span>
      )}
    </div>
  );
}


interface SpinnerProps {
  size?: "sm" | "md" | "lg";
  className?: string;
}

export function Spinner({ size = "md", className }: SpinnerProps) {
  const sizes = { sm: "w-4 h-4", md: "w-6 h-6", lg: "w-8 h-8" };
  return (
    <div
      className={cn(
        "border-2 border-emerald-500 border-t-transparent rounded-full animate-spin",
        sizes[size],
        className
      )}
    />
  );
}


interface EmptyStateProps {
  icon?: ReactNode;
  title: string;
  description?: string;
  action?: ReactNode;
  className?: string;
}

export function EmptyState({ icon, title, description, action, className }: EmptyStateProps) {
  return (
    <div className={cn("flex flex-col items-center justify-center py-12 text-center", className)}>
      {icon && <div className="mb-4 text-[#3a4558]">{icon}</div>}
      <h3 className="text-[14px] font-medium text-white mb-1">{title}</h3>
      {description && (
        <p className="text-[12px] text-[#8b95a5] mb-4 max-w-[280px]">{description}</p>
      )}
      {action}
    </div>
  );
}


interface LoadingStateProps {
  text?: string;
  size?: "sm" | "md" | "lg";
  className?: string;
}

export function LoadingState({ text = "Cargando...", size = "md", className }: LoadingStateProps) {
  return (
    <div className={cn("flex items-center justify-center gap-3", className)}>
      <Spinner size={size} />
      {text && <span className="text-[13px] text-[#8b95a5]">{text}</span>}
    </div>
  );
}


interface ErrorBannerProps {
  message: string;
  onDismiss?: () => void;
  className?: string;
}

export function ErrorBanner({ message, onDismiss, className }: ErrorBannerProps) {
  return (
    <div className={cn("p-3 rounded-lg bg-red-500/10 border border-red-500/30 flex items-center gap-2", className)}>
      <svg className="w-4 h-4 text-red-400 flex-shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <circle cx="12" cy="12" r="10" />
        <path d="M15 9l-6 6M9 9l6 6" />
      </svg>
      <span className="text-[13px] text-red-400 flex-1">{message}</span>
      {onDismiss && (
        <button onClick={onDismiss} className="text-red-400 hover:text-red-300 p-1">
          <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M18 6L6 18M6 6l12 12" />
          </svg>
        </button>
      )}
    </div>
  );
}


interface SuccessBannerProps {
  message: string;
  onDismiss?: () => void;
  className?: string;
}

export function SuccessBanner({ message, onDismiss, className }: SuccessBannerProps) {
  return (
    <div className={cn("p-3 rounded-lg bg-emerald-500/10 border border-emerald-500/30 flex items-center gap-2", className)}>
      <svg className="w-4 h-4 text-emerald-400 flex-shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
        <polyline points="22 4 12 14.01 9 11.01" />
      </svg>
      <span className="text-[13px] text-emerald-400 flex-1">{message}</span>
      {onDismiss && (
        <button onClick={onDismiss} className="text-emerald-400 hover:text-emerald-300 p-1">
          <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M18 6L6 18M6 6l12 12" />
          </svg>
        </button>
      )}
    </div>
  );
}

