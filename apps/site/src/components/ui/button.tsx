import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "../../lib/utils";

export const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 rounded-full text-sm font-semibold transition-[transform,background-color,color,border-color,box-shadow] duration-300 ease-out-quint focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-coral focus-visible:ring-offset-2 focus-visible:ring-offset-ink disabled:pointer-events-none disabled:opacity-50 active:translate-y-px",
  {
    variants: {
      variant: {
        default: "bg-coral text-ink shadow-[0_10px_35px_oklch(0.06_0.003_255/0.3)] hover:-translate-y-0.5 hover:bg-coral-bright",
        outline: "border border-line bg-transparent text-paper hover:-translate-y-0.5 hover:border-paper/35 hover:bg-paper/5",
        ghost: "text-paper-muted hover:bg-paper/6 hover:text-paper",
      },
      size: {
        default: "h-11 px-5",
        sm: "h-9 px-4 text-xs",
        lg: "h-14 px-7 text-base",
        icon: "size-11 p-0",
      },
    },
    defaultVariants: { variant: "default", size: "default" },
  },
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, ...props }, ref) => (
    <button ref={ref} className={cn(buttonVariants({ variant, size }), className)} {...props} />
  ),
);

Button.displayName = "Button";
