// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// @ts-nocheck
/**
 * Caesar Design System - Component Library
 * Professional, enterprise-grade React components
 */

import { forwardRef, ReactNode, ComponentProps } from 'react';
// import { designTokens } from './tokens';
import { cn } from './utils';

// Base Component Props
interface BaseProps {
  className?: string;
  children?: ReactNode;
}

// Typography Components
interface TextProps extends BaseProps {
  variant?: 'display' | 'h1' | 'h2' | 'h3' | 'h4' | 'body' | 'caption' | 'overline';
  weight?: 'light' | 'normal' | 'medium' | 'semibold' | 'bold';
  color?: 'primary' | 'secondary' | 'gold' | 'success' | 'warning' | 'danger';
  align?: 'left' | 'center' | 'right';
  as?: keyof JSX.IntrinsicElements;
}

export const Text = forwardRef<any, TextProps>(({
  variant = 'body',
  weight = 'normal',
  color = 'primary',
  align = 'left',
  as: Component = 'p',
  className,
  children,
  ...props
}, ref) => {
  const variantClasses = {
    display: 'text-6xl font-display font-bold leading-tight',
    h1: 'text-4xl font-semibold leading-tight',
    h2: 'text-3xl font-semibold leading-tight',
    h3: 'text-2xl font-semibold leading-normal',
    h4: 'text-xl font-medium leading-normal',
    body: 'text-base leading-normal',
    caption: 'text-sm leading-normal',
    overline: 'text-xs uppercase tracking-wider font-medium',
  };

  const weightClasses = {
    light: 'font-light',
    normal: 'font-normal',
    medium: 'font-medium',
    semibold: 'font-semibold',
    bold: 'font-bold',
  };

  const colorClasses = {
    primary: 'text-neutral-50',
    secondary: 'text-neutral-400',
    gold: 'text-caesar-gold',
    success: 'text-semantic-success',
    warning: 'text-semantic-warning',
    danger: 'text-semantic-danger',
  };

  const alignClasses = {
    left: 'text-left',
    center: 'text-center',
    right: 'text-right',
  };

  return (
    // @ts-ignore - complex union type resolution
    <Component
      ref={ref as any}
      className={cn(
        variantClasses[variant],
        weightClasses[weight],
        colorClasses[color],
        alignClasses[align],
        className
      )}
      {...props}
    >
      {children}
    </Component>
  );
});

// Advanced Card System
interface CardProps extends BaseProps {
  variant?: 'default' | 'glass' | 'elevated' | 'outlined' | 'gold' | 'terminal';
  padding?: 'none' | 'sm' | 'md' | 'lg' | 'xl';
  hover?: boolean;
  glow?: boolean;
  onClick?: () => void;
}

export const Card = forwardRef<HTMLDivElement, CardProps>(({
  variant = 'default',
  padding = 'md',
  hover = false,
  glow = false,
  onClick,
  className,
  children,
  ...props
}, ref) => {
  const variantClasses = {
    default: 'bg-background-secondary border border-neutral-800',
    glass: 'bg-background-glass backdrop-blur-xl border border-white/10',
    elevated: 'bg-background-secondary shadow-xl border border-neutral-700',
    outlined: 'bg-transparent border-2 border-neutral-600',
    gold: 'bg-gradient-to-br from-caesar-gold/5 to-transparent border border-caesar-gold/30',
    terminal: 'bg-neutral-950 border border-caesar-gold font-mono',
  };

  const paddingClasses = {
    none: '',
    sm: 'p-3',
    md: 'p-6',
    lg: 'p-8',
    xl: 'p-12',
  };

  const hoverClasses = hover ? 'transition-all duration-250 hover:scale-[1.02] hover:shadow-lg' : '';
  const glowClasses = glow ? 'shadow-goldGlow-md' : '';

  return (
    <div
      ref={ref}
      className={cn(
        'rounded-2xl overflow-hidden',
        variantClasses[variant],
        paddingClasses[padding],
        hoverClasses,
        glowClasses,
        onClick && 'cursor-pointer',
        className
      )}
      onClick={onClick}
      {...props}
    >
      {children}
    </div>
  );
});

// Professional Button System
interface ButtonProps extends ComponentProps<'button'> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'outline' | 'gold' | 'glass' | 'danger' | 'success';
  size?: 'xs' | 'sm' | 'md' | 'lg' | 'xl';
  loading?: boolean;
  icon?: ReactNode;
  iconPosition?: 'left' | 'right';
  fullWidth?: boolean;
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(({
  variant = 'primary',
  size = 'md',
  loading = false,
  icon,
  iconPosition = 'left',
  fullWidth = false,
  className,
  children,
  disabled,
  ...props
}, ref) => {
  const variantClasses = {
    primary: 'bg-caesar-gold text-neutral-950 hover:bg-caesar-goldLight disabled:bg-neutral-600',
    secondary: 'bg-neutral-700 text-neutral-50 hover:bg-neutral-600 disabled:bg-neutral-800',
    ghost: 'bg-transparent text-neutral-300 hover:bg-neutral-800 disabled:text-neutral-600',
    outline: 'bg-transparent border-2 border-neutral-600 text-neutral-300 hover:border-neutral-500',
    gold: 'bg-gradient-to-r from-caesar-gold to-caesar-goldLight text-neutral-950 shadow-goldGlow-sm hover:shadow-goldGlow-md',
    glass: 'bg-background-glass backdrop-blur-xl border border-white/10 text-neutral-300 hover:bg-white/5',
    danger: 'bg-semantic-danger text-white hover:bg-red-600 disabled:bg-neutral-600',
    success: 'bg-semantic-success text-white hover:bg-green-600 disabled:bg-neutral-600',
  };

  const sizeClasses = {
    xs: 'px-2 py-1 text-xs',
    sm: 'px-3 py-2 text-sm',
    md: 'px-4 py-3 text-base',
    lg: 'px-6 py-4 text-lg',
    xl: 'px-8 py-5 text-xl',
  };

  const isDisabled = disabled || loading;

  return (
    <button
      ref={ref}
      className={cn(
        'inline-flex items-center justify-center font-medium rounded-lg transition-all duration-250',
        'focus:outline-none focus:ring-2 focus:ring-caesar-gold/50 focus:ring-offset-2 focus:ring-offset-background-primary',
        variantClasses[variant],
        sizeClasses[size],
        fullWidth ? 'w-full' : '',
        isDisabled ? 'cursor-not-allowed opacity-50' : 'cursor-pointer',
        className
      )}
      disabled={isDisabled}
      {...props}
    >
      {loading && (
        <svg className="animate-spin -ml-1 mr-2 h-4 w-4" fill="none" viewBox="0 0 24 24">
          <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
          <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
        </svg>
      )}
      {icon && iconPosition === 'left' && !loading && (
        <span className="mr-2">{icon}</span>
      )}
      {children}
      {icon && iconPosition === 'right' && !loading && (
        <span className="ml-2">{icon}</span>
      )}
    </button>
  );
});

// Advanced Input System
interface InputProps extends ComponentProps<'input'> {
  label?: string;
  error?: string;
  hint?: string;
  leftIcon?: ReactNode;
  rightIcon?: ReactNode;
  variant?: 'default' | 'ghost' | 'terminal';
}

export const Input = forwardRef<HTMLInputElement, InputProps>(({
  label,
  error,
  hint,
  leftIcon,
  rightIcon,
  variant = 'default',
  className,
  ...props
}, ref) => {
  const variantClasses = {
    default: 'bg-background-secondary border-neutral-700 focus:border-caesar-gold',
    ghost: 'bg-transparent border-neutral-600 focus:border-neutral-400',
    terminal: 'bg-neutral-950 border-caesar-gold font-mono text-caesar-gold',
  };

  return (
    <div className="space-y-2">
      {label && (
        <Text variant="caption" weight="medium" color="secondary" as="label">
          {label}
        </Text>
      )}
      <div className="relative">
        {leftIcon && (
          <div className="absolute left-3 top-1/2 transform -translate-y-1/2 text-neutral-400">
            {leftIcon}
          </div>
        )}
        <input
          ref={ref}
          className={cn(
            'w-full px-4 py-3 rounded-lg border transition-colors duration-250',
            'focus:outline-none focus:ring-2 focus:ring-caesar-gold/30',
            'placeholder:text-neutral-500',
            leftIcon ? 'pl-10' : '',
            rightIcon ? 'pr-10' : '',
            error ? 'border-semantic-danger focus:border-semantic-danger' : '',
            variantClasses[variant],
            className
          )}
          {...props}
        />
        {rightIcon && (
          <div className="absolute right-3 top-1/2 transform -translate-y-1/2 text-neutral-400">
            {rightIcon}
          </div>
        )}
      </div>
      {error && (
        <Text variant="caption" color="danger">
          {error}
        </Text>
      )}
      {hint && !error && (
        <Text variant="caption" color="secondary">
          {hint}
        </Text>
      )}
    </div>
  );
});

// Professional Badge System
interface BadgeProps extends BaseProps {
  variant?: 'default' | 'gold' | 'success' | 'warning' | 'danger' | 'info';
  size?: 'sm' | 'md' | 'lg';
  dot?: boolean;
}

export const Badge = forwardRef<HTMLSpanElement, BadgeProps>(({
  variant = 'default',
  size = 'md',
  dot = false,
  className,
  children,
  ...props
}, ref) => {
  const variantClasses = {
    default: 'bg-neutral-700 text-neutral-300',
    gold: 'bg-caesar-gold/20 text-caesar-gold border border-caesar-gold/30',
    success: 'bg-green-500/20 text-green-400 border border-green-500/30',
    warning: 'bg-yellow-500/20 text-yellow-400 border border-yellow-500/30',
    danger: 'bg-red-500/20 text-red-400 border border-red-500/30',
    info: 'bg-blue-500/20 text-blue-400 border border-blue-500/30',
  };

  const sizeClasses = {
    sm: 'px-2 py-1 text-xs',
    md: 'px-3 py-1.5 text-sm',
    lg: 'px-4 py-2 text-base',
  };

  if (dot) {
    return (
      <span
        ref={ref}
        className={cn(
          'inline-block w-2 h-2 rounded-full',
          variantClasses[variant].split(' ')[0], // Just the background color
          className
        )}
        {...props}
      />
    );
  }

  return (
    <span
      ref={ref}
      className={cn(
        'inline-flex items-center rounded-full font-medium',
        variantClasses[variant],
        sizeClasses[size],
        className
      )}
      {...props}
    >
      {children}
    </span>
  );
});

// Advanced Progress System
interface ProgressProps extends BaseProps {
  value: number;
  max?: number;
  variant?: 'default' | 'gold' | 'success' | 'warning' | 'danger';
  size?: 'sm' | 'md' | 'lg';
  showLabel?: boolean;
  animated?: boolean;
}

export const Progress = forwardRef<HTMLDivElement, ProgressProps>(({
  value,
  max = 100,
  variant = 'default',
  size = 'md',
  showLabel = false,
  animated = false,
  className,
  ...props
}, ref) => {
  const percentage = Math.min(100, Math.max(0, (value / max) * 100));

  const variantClasses = {
    default: 'bg-neutral-600',
    gold: 'bg-gradient-to-r from-caesar-gold to-caesar-goldLight',
    success: 'bg-semantic-success',
    warning: 'bg-semantic-warning',
    danger: 'bg-semantic-danger',
  };

  const sizeClasses = {
    sm: 'h-1',
    md: 'h-2',
    lg: 'h-3',
  };

  return (
    <div ref={ref} className={cn('w-full', className)} {...props}>
      {showLabel && (
        <div className="flex justify-between mb-2">
          <Text variant="caption" color="secondary">Progress</Text>
          <Text variant="caption" color="secondary">{percentage.toFixed(0)}%</Text>
        </div>
      )}
      <div className={cn('bg-neutral-800 rounded-full overflow-hidden', sizeClasses[size])}>
        <div
          className={cn(
            'h-full transition-all duration-500 rounded-full',
            animated ? 'bg-gradient-to-r animate-pulse' : '',
            variantClasses[variant]
          )}
          style={{ width: `${percentage}%` }}
        />
      </div>
    </div>
  );
});

// Professional Separator
interface SeparatorProps extends BaseProps {
  orientation?: 'horizontal' | 'vertical';
  variant?: 'default' | 'gold' | 'dashed';
}

export const Separator = forwardRef<HTMLDivElement, SeparatorProps>(({
  orientation = 'horizontal',
  variant = 'default',
  className,
  ...props
}, ref) => {
  const orientationClasses = {
    horizontal: 'w-full h-px',
    vertical: 'h-full w-px',
  };

  const variantClasses = {
    default: 'bg-neutral-700',
    gold: 'bg-gradient-to-r from-transparent via-caesar-gold to-transparent',
    dashed: 'bg-neutral-700 bg-gradient-to-r from-neutral-700 via-transparent to-neutral-700',
  };

  return (
    <div
      ref={ref}
      className={cn(
        orientationClasses[orientation],
        variantClasses[variant],
        className
      )}
      {...props}
    />
  );
});

export { cn } from './utils';