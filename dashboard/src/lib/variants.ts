import { cva, type VariantProps } from "class-variance-authority";

export const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 font-medium transition-colors focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-[#0a0e17] disabled:opacity-50 disabled:cursor-not-allowed",
  {
    variants: {
      variant: {
        primary: "bg-emerald-500 hover:bg-emerald-600 text-[#0a0e17] focus:ring-emerald-500",
        secondary: "bg-[#1a2332] hover:bg-[#2a3548] text-white border border-[#2a3548] focus:ring-[#2a3548]",
        ghost: "hover:bg-[#1a2332] text-[#8b95a5] hover:text-white",
        danger: "bg-red-500/20 hover:bg-red-500/30 text-red-400 focus:ring-red-500",
        accent: "bg-purple-500/20 hover:bg-purple-500/30 text-purple-400 focus:ring-purple-500",
        link: "text-emerald-400 hover:text-emerald-300 underline-offset-4 hover:underline",
      },
      size: {
        xs: "text-[11px] px-2 py-1 rounded",
        sm: "text-[12px] px-3 py-1.5 rounded-md",
        md: "text-[13px] px-4 py-2 rounded-lg",
        lg: "text-[14px] px-5 py-2.5 rounded-lg",
      },
    },
    defaultVariants: {
      variant: "primary",
      size: "md",
    },
  }
);

export type ButtonVariants = VariantProps<typeof buttonVariants>;

export const inputVariants = cva(
  "w-full border bg-[#0a0e17] text-white placeholder-[#5a6578] focus:outline-none focus:ring-2 transition-colors",
  {
    variants: {
      variant: {
        default: "border-[#2a3548] focus:ring-emerald-500 focus:border-emerald-500",
        error: "border-red-500 focus:ring-red-500",
        success: "border-emerald-500 focus:ring-emerald-500",
      },
      size: {
        sm: "text-[12px] px-2.5 py-1.5 rounded-md",
        md: "text-[13px] px-3 py-2 rounded-lg",
        lg: "text-[14px] px-4 py-2.5 rounded-lg",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "md",
    },
  }
);

export type InputVariants = VariantProps<typeof inputVariants>;

export const cardVariants = cva(
  "rounded-lg border transition-colors",
  {
    variants: {
      variant: {
        default: "bg-[#131a26] border-[#2a3548]",
        elevated: "bg-[#0a0e17] border-[#2a3548]",
        interactive: "bg-[#0a0e17] border-[#2a3548] hover:border-emerald-400 hover:shadow-[0_0_0_1px_#00d4aa] cursor-pointer",
        selected: "bg-emerald-500/10 border-emerald-500/50",
        warning: "bg-amber-500/10 border-amber-500/30",
        error: "bg-red-500/10 border-red-500/30",
        success: "bg-emerald-500/10 border-emerald-500/30",
      },
      padding: {
        none: "",
        sm: "p-3",
        md: "p-4",
        lg: "p-5",
      },
    },
    defaultVariants: {
      variant: "default",
      padding: "md",
    },
  }
);

export type CardVariants = VariantProps<typeof cardVariants>;

export const badgeVariants = cva(
  "inline-flex items-center font-medium rounded-full",
  {
    variants: {
      variant: {
        default: "bg-[#2a3548] text-[#8b95a5]",
        success: "bg-emerald-500/20 text-emerald-400",
        warning: "bg-amber-500/20 text-amber-400",
        error: "bg-red-500/20 text-red-400",
        info: "bg-blue-500/20 text-blue-400",
        purple: "bg-purple-500/20 text-purple-400",
        cyan: "bg-cyan-500/20 text-cyan-400",
      },
      size: {
        xs: "text-[9px] px-1.5 py-0.5",
        sm: "text-[10px] px-2 py-0.5",
        md: "text-[11px] px-2.5 py-1",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "sm",
    },
  }
);

export type BadgeVariants = VariantProps<typeof badgeVariants>;

export const labelVariants = cva(
  "block uppercase tracking-wide",
  {
    variants: {
      variant: {
        default: "text-[#8b95a5]",
        muted: "text-[#5a6578]",
      },
      size: {
        sm: "text-[10px]",
        md: "text-[12px]",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "md",
    },
  }
);

export type LabelVariants = VariantProps<typeof labelVariants>;

export const statusDotVariants = cva(
  "rounded-full",
  {
    variants: {
      status: {
        online: "bg-emerald-500",
        offline: "bg-red-500",
        warning: "bg-amber-500",
        unknown: "bg-gray-500",
      },
      size: {
        sm: "w-1.5 h-1.5",
        md: "w-2 h-2",
        lg: "w-2.5 h-2.5",
      },
    },
    defaultVariants: {
      status: "unknown",
      size: "md",
    },
  }
);

export type StatusDotVariants = VariantProps<typeof statusDotVariants>;

export const progressBarVariants = cva(
  "h-full rounded-full transition-all duration-300",
  {
    variants: {
      variant: {
        cpu: "bg-gradient-to-r from-emerald-400 to-indigo-500",
        memory: "bg-gradient-to-r from-indigo-500 to-purple-500",
        disk: "bg-gradient-to-r from-purple-500 to-pink-500",
        network: "bg-gradient-to-r from-cyan-400 to-blue-500",
        danger: "bg-gradient-to-r from-amber-500 to-red-500",
        success: "bg-emerald-500",
      },
    },
    defaultVariants: {
      variant: "success",
    },
  }
);

export type ProgressBarVariants = VariantProps<typeof progressBarVariants>;

export const iconButtonVariants = cva(
  "inline-flex items-center justify-center rounded-md transition-colors focus:outline-none",
  {
    variants: {
      variant: {
        ghost: "text-[#8b95a5] hover:text-white hover:bg-[#1a2332]",
        danger: "text-[#8b95a5] hover:text-red-400 hover:bg-red-500/10",
        accent: "text-emerald-400 hover:bg-emerald-500/10",
      },
      size: {
        sm: "w-6 h-6",
        md: "w-8 h-8",
        lg: "w-10 h-10",
      },
    },
    defaultVariants: {
      variant: "ghost",
      size: "md",
    },
  }
);

export type IconButtonVariants = VariantProps<typeof iconButtonVariants>;
