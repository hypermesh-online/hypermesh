// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// @ts-nocheck
/**
 * Caesar Design System - Professional Typography & Spacing System
 * Enterprise-grade typography components with perfect spacing
 */

import React, { forwardRef, ReactNode, ComponentProps } from 'react';
import { cn } from './utils';

// Typography Scale
export const typographyScale = {
  display: {
    fontSize: '3.75rem', // 60px
    lineHeight: '1.2',
    fontWeight: '700',
    letterSpacing: '-0.025em',
  },
  h1: {
    fontSize: '2.25rem', // 36px
    lineHeight: '1.25',
    fontWeight: '600',
    letterSpacing: '-0.025em',
  },
  h2: {
    fontSize: '1.875rem', // 30px
    lineHeight: '1.25',
    fontWeight: '600',
    letterSpacing: '-0.025em',
  },
  h3: {
    fontSize: '1.5rem', // 24px
    lineHeight: '1.33',
    fontWeight: '600',
    letterSpacing: '0',
  },
  h4: {
    fontSize: '1.25rem', // 20px
    lineHeight: '1.4',
    fontWeight: '500',
    letterSpacing: '0',
  },
  h5: {
    fontSize: '1.125rem', // 18px
    lineHeight: '1.4',
    fontWeight: '500',
    letterSpacing: '0',
  },
  h6: {
    fontSize: '1rem', // 16px
    lineHeight: '1.5',
    fontWeight: '500',
    letterSpacing: '0',
  },
  body: {
    fontSize: '1rem', // 16px
    lineHeight: '1.5',
    fontWeight: '400',
    letterSpacing: '0',
  },
  bodyLarge: {
    fontSize: '1.125rem', // 18px
    lineHeight: '1.5',
    fontWeight: '400',
    letterSpacing: '0',
  },
  bodyMedium: {
    fontSize: '1.0625rem', // 17px
    lineHeight: '1.47',
    fontWeight: '400',
    letterSpacing: '0',
  },
  bodySmall: {
    fontSize: '0.875rem', // 14px
    lineHeight: '1.43',
    fontWeight: '400',
    letterSpacing: '0',
  },
  caption: {
    fontSize: '0.75rem', // 12px
    lineHeight: '1.5',
    fontWeight: '400',
    letterSpacing: '0.025em',
  },
  overline: {
    fontSize: '0.75rem', // 12px
    lineHeight: '1.5',
    fontWeight: '500',
    letterSpacing: '0.1em',
    textTransform: 'uppercase' as const,
  },
  code: {
    fontSize: '0.875rem', // 14px
    lineHeight: '1.43',
    fontWeight: '400',
    letterSpacing: '0',
    fontFamily: 'var(--font-mono)',
  },
} as const;

// Enhanced Typography Component
interface TypographyProps extends ComponentProps<'p'> {
  variant?: keyof typeof typographyScale;
  color?: 'primary' | 'secondary' | 'tertiary' | 'gold' | 'success' | 'warning' | 'danger' | 'info';
  align?: 'left' | 'center' | 'right' | 'justify';
  weight?: 'light' | 'normal' | 'medium' | 'semibold' | 'bold' | 'extrabold';
  family?: 'primary' | 'mono' | 'display';
  spacing?: 'tight' | 'normal' | 'relaxed' | 'loose';
  truncate?: boolean;
  italic?: boolean;
  underline?: boolean;
  gradient?: boolean;
  glow?: boolean;
  animate?: boolean;
  as?: keyof JSX.IntrinsicElements;
}

export const Typography = forwardRef<any, TypographyProps>((
  {
    variant = 'body',
    color = 'primary',
    align = 'left',
    weight,
    family,
    spacing,
    truncate = false,
    italic = false,
    underline = false,
    gradient = false,
    glow = false,
    animate = false,
    as,
    className,
    children,
    ...props
  },
  ref
) => {
  const scale = typographyScale[variant];
  
  // Determine the HTML element
  const Component = as || 
    (variant === 'display' ? 'h1' :
     variant.startsWith('h') ? variant as keyof JSX.IntrinsicElements :
     variant === 'overline' ? 'span' :
     variant === 'caption' ? 'span' :
     variant === 'code' ? 'code' : 'p') as keyof JSX.IntrinsicElements;

  const colorClasses = {
    primary: 'text-neutral-50',
    secondary: 'text-neutral-400',
    tertiary: 'text-neutral-600',
    gold: 'text-caesar-gold',
    success: 'text-semantic-success',
    warning: 'text-semantic-warning',
    danger: 'text-semantic-danger',
    info: 'text-semantic-info',
  };

  const alignClasses = {
    left: 'text-left',
    center: 'text-center',
    right: 'text-right',
    justify: 'text-justify',
  };

  const weightClasses = {
    light: 'font-light',
    normal: 'font-normal',
    medium: 'font-medium',
    semibold: 'font-semibold',
    bold: 'font-bold',
    extrabold: 'font-extrabold',
  };

  const familyClasses = {
    primary: 'font-sans',
    mono: 'font-mono',
    display: 'font-display',
  };

  const spacingClasses = {
    tight: 'tracking-tight',
    normal: 'tracking-normal',
    relaxed: 'tracking-wide',
    loose: 'tracking-wider',
  };

  return (
    <Component
      ref={ref as any}
      className={cn(
        // Base typography scale
        'transition-all duration-250',
        
        // Color
        gradient ? 'text-gradient bg-gradient-to-r from-caesar-gold to-caesar-goldLight bg-clip-text text-transparent' : colorClasses[color],
        
        // Alignment
        alignClasses[align],
        
        // Weight override
        weight && weightClasses[weight],
        
        // Font family override
        family && familyClasses[family],
        
        // Letter spacing override
        spacing && spacingClasses[spacing],
        
        // Special effects
        truncate && 'truncate',
        italic && 'italic',
        underline && 'underline decoration-caesar-gold',
        glow && 'drop-shadow-goldGlow-sm',
        animate && 'hover:scale-105 hover:text-caesar-goldLight cursor-default',
        
        className
      )}
      style={{
        fontSize: scale.fontSize,
        lineHeight: scale.lineHeight,
        fontWeight: weight ? undefined : scale.fontWeight,
        letterSpacing: spacing ? undefined : scale.letterSpacing,
        fontFamily: family ? undefined : 'fontFamily' in scale ? scale.fontFamily : undefined,
        textTransform: 'textTransform' in scale ? scale.textTransform : undefined,
      }}
      {...props}
    >
      {children}
    </Component>
  );
});

// @ts-ignore - SVG type mismatch
Typography.displayName = 'Typography';

// Spacing System Components
interface SpacerProps {
  size?: 'xs' | 'sm' | 'md' | 'lg' | 'xl' | '2xl' | '3xl' | '4xl';
  direction?: 'vertical' | 'horizontal';
}

export const Spacer: React.FC<SpacerProps> = ({ 
  size = 'md', 
  direction = 'vertical' 
}) => {
  const sizeClasses = {
    xs: direction === 'vertical' ? 'h-1' : 'w-1',
    sm: direction === 'vertical' ? 'h-2' : 'w-2',
    md: direction === 'vertical' ? 'h-4' : 'w-4',
    lg: direction === 'vertical' ? 'h-6' : 'w-6',
    xl: direction === 'vertical' ? 'h-8' : 'w-8',
    '2xl': direction === 'vertical' ? 'h-12' : 'w-12',
    '3xl': direction === 'vertical' ? 'h-16' : 'w-16',
    '4xl': direction === 'vertical' ? 'h-24' : 'w-24',
  };

  return <div className={sizeClasses[size]} />;
};

// Text Block Component for Rich Content
interface TextBlockProps extends ComponentProps<'div'> {
  title?: string;
  subtitle?: string;
  description?: string;
  spacing?: 'compact' | 'normal' | 'loose';
  align?: 'left' | 'center' | 'right';
  maxWidth?: 'sm' | 'md' | 'lg' | 'xl' | '2xl' | 'none';
}

export const TextBlock = forwardRef<HTMLDivElement, TextBlockProps>((
  {
    title,
    subtitle,
    description,
    spacing = 'normal',
    align = 'left',
    maxWidth = 'none',
    className,
    children,
    ...props
  },
  ref
) => {
  const spacingClasses = {
    compact: 'space-y-2',
    normal: 'space-y-4',
    loose: 'space-y-6',
  };

  const alignClasses = {
    left: 'text-left',
    center: 'text-center',
    right: 'text-right',
  };

  const maxWidthClasses = {
    sm: 'max-w-sm',
    md: 'max-w-md',
    lg: 'max-w-lg',
    xl: 'max-w-xl',
    '2xl': 'max-w-2xl',
    none: '',
  };

  return (
    <div
      ref={ref}
      className={cn(
        spacingClasses[spacing],
        alignClasses[align],
        maxWidthClasses[maxWidth],
        className
      )}
      {...props}
    >
      {title && (
        <Typography variant="h2" weight="semibold">
          {title}
        </Typography>
      )}
      {subtitle && (
        <Typography variant="h4" color="secondary" weight="medium">
          {subtitle}
        </Typography>
      )}
      {description && (
        <Typography variant="bodyLarge" color="secondary">
          {description}
        </Typography>
      )}
      {children}
    </div>
  );
});

TextBlock.displayName = 'TextBlock';

// Code Block Component
interface CodeBlockProps extends ComponentProps<'pre'> {
  language?: string;
  showLineNumbers?: boolean;
  highlight?: number[];
  copy?: boolean;
}

export const CodeBlock = forwardRef<HTMLPreElement, CodeBlockProps>((
  {
    language = 'text',
    showLineNumbers = false,
    highlight = [],
    copy = true,
    className,
    children,
    ...props
  },
  ref
) => {
  const [copied, setCopied] = React.useState(false);

  const handleCopy = async () => {
    if (typeof children === 'string') {
      try {
        await navigator.clipboard.writeText(children);
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
      } catch (err) {
        console.error('Failed to copy:', err);
      }
    }
  };

  const lines = typeof children === 'string' ? children.split('\n') : [];

  return (
    <div className="relative group">
      <pre
        ref={ref}
        className={cn(
          'bg-neutral-950 border border-caesar-gold/30 rounded-lg p-4 overflow-x-auto',
          'font-mono text-sm text-caesar-gold',
          'scrollbar-thin scrollbar-thumb-caesar-gold/50 scrollbar-track-transparent',
          className
        )}
        {...props}
      >
        {showLineNumbers && lines.length > 1 ? (
          <div className="flex">
            <div className="text-neutral-600 text-right pr-4 select-none min-w-8">
              {lines.map((_, index) => (
                <div
                  key={index}
                  className={cn(
                    'leading-relaxed',
                    highlight.includes(index + 1) && 'text-caesar-gold'
                  )}
                >
                  {index + 1}
                </div>
              ))}
            </div>
            <div className="flex-1">
              {lines.map((line, index) => (
                <div
                  key={index}
                  className={cn(
                    'leading-relaxed',
                    highlight.includes(index + 1) && 'bg-caesar-gold/10 -mx-2 px-2'
                  )}
                >
                  {line || ' '}
                </div>
              ))}
            </div>
          </div>
        ) : (
          <code>{children}</code>
        )}
      </pre>
      
      {copy && (
        <button
          onClick={handleCopy}
          className={cn(
            'absolute top-2 right-2 p-2 rounded-md transition-all duration-200',
            'bg-neutral-800 hover:bg-neutral-700 text-neutral-400 hover:text-white',
            'opacity-0 group-hover:opacity-100',
            copied && 'text-semantic-success'
          )}
          title={copied ? 'Copied!' : 'Copy code'}
        >
          {copied ? '✓' : '📋'}
        </button>
      )}
      
      {language && (
        <div className="absolute top-2 left-2 px-2 py-1 bg-neutral-800 rounded text-xs text-neutral-400 font-mono">
          {language}
        </div>
      )}
    </div>
  );
});

CodeBlock.displayName = 'CodeBlock';

// Inline Code Component
interface InlineCodeProps extends ComponentProps<'code'> {
  variant?: 'default' | 'success' | 'warning' | 'danger';
}

export const InlineCode = forwardRef<HTMLElement, InlineCodeProps>((
  {
    variant = 'default',
    className,
    children,
    ...props
  },
  ref
) => {
  const variantClasses = {
    default: 'bg-neutral-800 text-caesar-gold border-neutral-700',
    success: 'bg-semantic-success/20 text-semantic-success border-semantic-success/30',
    warning: 'bg-semantic-warning/20 text-semantic-warning border-semantic-warning/30',
    danger: 'bg-semantic-danger/20 text-semantic-danger border-semantic-danger/30',
  };

  return (
    <code
      ref={ref}
      className={cn(
        'px-1.5 py-0.5 rounded border font-mono text-sm',
        variantClasses[variant],
        className
      )}
      {...props}
    >
      {children}
    </code>
  );
});

InlineCode.displayName = 'InlineCode';

// Quote Component
interface QuoteProps extends ComponentProps<'blockquote'> {
  author?: string;
  source?: string;
  variant?: 'default' | 'emphasized' | 'minimal';
}

export const Quote = forwardRef<HTMLQuoteElement, QuoteProps>((
  {
    author,
    source,
    variant = 'default',
    className,
    children,
    ...props
  },
  ref
) => {
  const variantClasses = {
    default: 'border-l-4 border-caesar-gold pl-6 py-4 bg-neutral-900/50',
    emphasized: 'border border-caesar-gold/30 p-6 bg-gradient-to-r from-caesar-gold/5 to-transparent',
    minimal: 'italic text-neutral-300',
  };

  return (
    <blockquote
      ref={ref}
      className={cn(
        'relative',
        variantClasses[variant],
        className
      )}
      {...props}
    >
      {variant !== 'minimal' && (
        <div className="absolute top-2 left-2 text-4xl text-caesar-gold/30 font-display">
          "
        </div>
      )}
      
      <div className={variant !== 'minimal' ? 'ml-6' : ''}>
        <Typography variant="bodyLarge" italic={variant === 'minimal'}>
          {children}
        </Typography>
        
        {(author || source) && (
          <footer className="mt-4">
            <Typography variant="bodySmall" color="secondary">
              {author && <cite className="not-italic">— {author}</cite>}
              {source && <span className="ml-2 text-neutral-500">({source})</span>}
            </Typography>
          </footer>
        )}
      </div>
    </blockquote>
  );
});

Quote.displayName = 'Quote';

// List Component
interface ListProps extends ComponentProps<'ul'> {
  ordered?: boolean;
  spacing?: 'compact' | 'normal' | 'loose';
  marker?: 'default' | 'none' | 'custom';
  customMarker?: ReactNode;
  variant?: 'default' | 'minimal' | 'enhanced';
}

export const List = forwardRef<HTMLUListElement | HTMLOListElement, ListProps>((
  {
    ordered = false,
    spacing = 'normal',
    marker = 'default',
    customMarker,
    variant = 'default',
    className,
    children,
    ...props
  },
  ref
) => {
  const Component = ordered ? 'ol' : 'ul';
  
  const spacingClasses = {
    compact: 'space-y-1',
    normal: 'space-y-2',
    loose: 'space-y-4',
  };

  const markerClasses = {
    default: ordered ? 'list-decimal' : 'list-disc',
    none: 'list-none',
    custom: 'list-none',
  };

  const variantClasses = {
    default: 'pl-6',
    minimal: 'pl-4',
    enhanced: 'pl-8 border-l border-neutral-700',
  };

  return (
    <Component
      ref={ref as any}
      className={cn(
        spacingClasses[spacing],
        markerClasses[marker],
        variantClasses[variant],
        marker === 'default' && 'marker:text-caesar-gold',
        className
      )}
      {...props}
    >
      {marker === 'custom' && customMarker
        ? React.Children.map(children, (child, index) => (
            <li key={index} className="flex items-start gap-3">
              <span className="flex-shrink-0 mt-1">{customMarker}</span>
              <span className="flex-1">{child}</span>
            </li>
          ))
        : children}
    </Component>
  );
});

List.displayName = 'List';