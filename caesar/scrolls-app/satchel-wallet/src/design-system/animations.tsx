// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/**
 * Caesar Design System - Advanced Animation & Interaction Patterns
 * Professional animation components for enterprise-grade UI
 */

import React, { forwardRef, ReactNode, ComponentProps, useState, useEffect } from 'react';
import { cn } from './utils';

// Animation Wrapper Component
interface AnimatedProps extends ComponentProps<'div'> {
  animation?: 'fadeIn' | 'fadeUp' | 'scaleIn' | 'slideInLeft' | 'slideInRight' | 'slideDown' | 'glowPulse' | 'bounce';
  delay?: number;
  duration?: number;
  trigger?: 'immediate' | 'hover' | 'focus' | 'visible';
  repeat?: boolean;
  easing?: 'linear' | 'ease' | 'ease-in' | 'ease-out' | 'ease-in-out';
}

export const Animated = forwardRef<HTMLDivElement, AnimatedProps>((
  {
    animation = 'fadeIn',
    delay = 0,
    duration = 300,
    trigger = 'immediate',
    repeat = false,
    easing = 'ease-out',
    className,
    children,
    style,
    ...props
  },
  ref
) => {
  const [isVisible] = useState(trigger === 'immediate');
  const [isTriggered, setIsTriggered] = useState(false);

  const animationClasses = {
    fadeIn: 'fade-in',
    fadeUp: 'fade-up',
    scaleIn: 'scale-in',
    slideInLeft: 'slide-in-left',
    slideInRight: 'slide-in-right',
    slideDown: 'slide-down',
    glowPulse: 'glow-pulse',
    bounce: 'bounce',
  };

  const handleTrigger = () => {
    if (trigger === 'hover' || trigger === 'focus') {
      setIsTriggered(true);
      if (!repeat) {
        setTimeout(() => setIsTriggered(false), duration);
      }
    }
  };

  const animationStyle = {
    animationDelay: `${delay}ms`,
    animationDuration: `${duration}ms`,
    animationTimingFunction: easing,
    animationIterationCount: repeat ? 'infinite' : '1',
    ...style,
  };

  return (
    <div
      ref={ref}
      className={cn(
        (isVisible || isTriggered) && animationClasses[animation],
        className
      )}
      style={animationStyle}
      onMouseEnter={trigger === 'hover' ? handleTrigger : undefined}
      onFocus={trigger === 'focus' ? handleTrigger : undefined}
      {...props}
    >
      {children}
    </div>
  );
});

Animated.displayName = 'Animated';

// Staggered Animation Container
interface StaggeredProps extends ComponentProps<'div'> {
  staggerDelay?: number;
  animation?: AnimatedProps['animation'];
  children: ReactNode[];
}

export const Staggered = forwardRef<HTMLDivElement, StaggeredProps>((
  {
    staggerDelay = 100,
    animation = 'fadeUp',
    className,
    children,
    ...props
  },
  ref
) => {
  return (
    <div ref={ref} className={className} {...props}>
      {React.Children.map(children, (child, index) => (
        <Animated
          key={index}
          animation={animation}
          delay={index * staggerDelay}
        >
          {child}
        </Animated>
      ))}
    </div>
  );
});

Staggered.displayName = 'Staggered';

// Hover Effects Component
interface HoverEffectProps extends ComponentProps<'div'> {
  effect?: 'lift' | 'glow' | 'scale' | 'rotate' | 'shake' | 'pulse';
  intensity?: 'subtle' | 'normal' | 'strong';
}

export const HoverEffect = forwardRef<HTMLDivElement, HoverEffectProps>((
  {
    effect = 'lift',
    intensity = 'normal',
    className,
    children,
    ...props
  },
  ref
) => {
  const effectClasses = {
    lift: {
      subtle: 'hover:-translate-y-0.5 transition-transform duration-200',
      normal: 'hover:-translate-y-1 transition-transform duration-200',
      strong: 'hover:-translate-y-2 transition-transform duration-200',
    },
    glow: {
      subtle: 'hover:shadow-goldGlow-sm transition-shadow duration-300',
      normal: 'hover:shadow-goldGlow-md transition-shadow duration-300',
      strong: 'hover:shadow-goldGlow-lg transition-shadow duration-300',
    },
    scale: {
      subtle: 'hover:scale-[1.02] transition-transform duration-200',
      normal: 'hover:scale-105 transition-transform duration-200',
      strong: 'hover:scale-110 transition-transform duration-200',
    },
    rotate: {
      subtle: 'hover:rotate-1 transition-transform duration-200',
      normal: 'hover:rotate-3 transition-transform duration-200',
      strong: 'hover:rotate-6 transition-transform duration-200',
    },
    shake: {
      subtle: 'hover:animate-pulse transition-all duration-200',
      normal: 'hover:animate-bounce transition-all duration-200',
      strong: 'hover:animate-ping transition-all duration-200',
    },
    pulse: {
      subtle: 'hover:opacity-80 transition-opacity duration-200',
      normal: 'hover:opacity-70 transition-opacity duration-200',
      strong: 'hover:opacity-60 transition-opacity duration-200',
    },
  };

  return (
    <div
      ref={ref}
      className={cn(
        effectClasses[effect][intensity],
        className
      )}
      {...props}
    >
      {children}
    </div>
  );
});

HoverEffect.displayName = 'HoverEffect';

// Loading States Component
interface LoadingProps extends ComponentProps<'div'> {
  type?: 'spinner' | 'dots' | 'pulse' | 'skeleton' | 'bars';
  size?: 'sm' | 'md' | 'lg' | 'xl';
  color?: 'gold' | 'white' | 'gray';
}

export const Loading = forwardRef<HTMLDivElement, LoadingProps>((
  {
    type = 'spinner',
    size = 'md',
    color = 'gold',
    className,
    ...props
  },
  ref
) => {
  const sizeClasses = {
    sm: 'w-4 h-4',
    md: 'w-6 h-6',
    lg: 'w-8 h-8',
    xl: 'w-12 h-12',
  };

  const colorClasses = {
    gold: 'text-caesar-gold',
    white: 'text-white',
    gray: 'text-neutral-400',
  };

  if (type === 'spinner') {
    return (
      <div
        ref={ref}
        className={cn(
          'animate-spin rounded-full border-2 border-transparent',
          'border-t-current border-r-current',
          sizeClasses[size],
          colorClasses[color],
          className
        )}
        {...props}
      />
    );
  }

  if (type === 'dots') {
    return (
      <div
        ref={ref}
        className={cn('flex space-x-1', className)}
        {...props}
      >
        {[0, 1, 2].map((i) => (
          <div
            key={i}
            className={cn(
              'rounded-full animate-bounce',
              sizeClasses[size],
              colorClasses[color] === 'text-caesar-gold' ? 'bg-caesar-gold' :
              colorClasses[color] === 'text-white' ? 'bg-white' : 'bg-neutral-400'
            )}
            style={{
              animationDelay: `${i * 0.1}s`,
            }}
          />
        ))}
      </div>
    );
  }

  if (type === 'pulse') {
    return (
      <div
        ref={ref}
        className={cn(
          'animate-pulse rounded-lg bg-neutral-700',
          size === 'sm' ? 'h-16' :
          size === 'md' ? 'h-24' :
          size === 'lg' ? 'h-32' : 'h-48',
          className
        )}
        {...props}
      />
    );
  }

  if (type === 'skeleton') {
    return (
      <div
        ref={ref}
        className={cn('animate-pulse space-y-3', className)}
        {...props}
      >
        <div className="h-4 bg-neutral-700 rounded w-3/4"></div>
        <div className="h-4 bg-neutral-700 rounded w-1/2"></div>
        <div className="h-4 bg-neutral-700 rounded w-5/6"></div>
      </div>
    );
  }

  if (type === 'bars') {
    return (
      <div
        ref={ref}
        className={cn('flex space-x-1', className)}
        {...props}
      >
        {[0, 1, 2, 3, 4].map((i) => (
          <div
            key={i}
            className={cn(
              'w-1 animate-pulse rounded-full',
              colorClasses[color] === 'text-caesar-gold' ? 'bg-caesar-gold' :
              colorClasses[color] === 'text-white' ? 'bg-white' : 'bg-neutral-400',
              size === 'sm' ? 'h-6' :
              size === 'md' ? 'h-8' :
              size === 'lg' ? 'h-12' : 'h-16'
            )}
            style={{
              animationDelay: `${i * 0.1}s`,
              animationDuration: '1s',
            }}
          />
        ))}
      </div>
    );
  }

  return null;
});

Loading.displayName = 'Loading';

// Parallax Component
interface ParallaxProps extends ComponentProps<'div'> {
  offset?: number;
  speed?: number;
}

export const Parallax = forwardRef<HTMLDivElement, ParallaxProps>((
  {
    offset = 0,
    speed = 0.5,
    className,
    children,
    style,
    ...props
  },
  ref
) => {
  const [scrollY, setScrollY] = useState(0);

  useEffect(() => {
    const handleScroll = () => setScrollY(window.scrollY);
    window.addEventListener('scroll', handleScroll);
    return () => window.removeEventListener('scroll', handleScroll);
  }, []);

  const parallaxStyle = {
    transform: `translateY(${(scrollY - offset) * speed}px)`,
    ...style,
  };

  return (
    <div
      ref={ref}
      className={className}
      style={parallaxStyle}
      {...props}
    >
      {children}
    </div>
  );
});

Parallax.displayName = 'Parallax';

// Reveal Component (Intersection Observer)
interface RevealProps extends ComponentProps<'div'> {
  animation?: AnimatedProps['animation'];
  delay?: number;
  threshold?: number;
  once?: boolean;
}

export const Reveal = forwardRef<HTMLDivElement, RevealProps>((
  {
    animation = 'fadeUp',
    delay = 0,
    threshold = 0.1,
    once = true,
    className,
    children,
    ...props
  },
  ref
) => {
  const [isVisible, setIsVisible] = useState(false);
  const [element, setElement] = useState<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!element) return;

    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setIsVisible(true);
          if (once) {
            observer.unobserve(element);
          }
        } else if (!once) {
          setIsVisible(false);
        }
      },
      { threshold }
    );

    observer.observe(element);
    return () => observer.disconnect();
  }, [element, threshold, once]);

  return (
    <Animated
      ref={(node) => {
        if (typeof ref === 'function') ref(node);
        else if (ref) ref.current = node;
        setElement(node);
      }}
      animation={animation}
      delay={delay}
      trigger={isVisible ? 'immediate' : undefined}
      className={className}
      {...props}
    >
      {children}
    </Animated>
  );
});

Reveal.displayName = 'Reveal';

// Transition Component
interface TransitionProps extends ComponentProps<'div'> {
  show?: boolean;
  enter?: string;
  enterFrom?: string;
  enterTo?: string;
  leave?: string;
  leaveFrom?: string;
  leaveTo?: string;
  duration?: number;
}

export const Transition = forwardRef<HTMLDivElement, TransitionProps>((
  {
    show = true,
    enter = 'transition-all duration-300',
    enterFrom = 'opacity-0 scale-95',
    enterTo = 'opacity-100 scale-100',
    leave = 'transition-all duration-200',
    leaveFrom = 'opacity-100 scale-100',
    leaveTo = 'opacity-0 scale-95',
    duration = 300,
    className,
    children,
    ...props
  },
  ref
) => {
  const [isVisible, setIsVisible] = useState(show);
  const [shouldRender, setShouldRender] = useState(show);

  useEffect(() => {
    if (show) {
      setShouldRender(true);
      setTimeout(() => setIsVisible(true), 10);
    } else {
      setIsVisible(false);
      setTimeout(() => setShouldRender(false), duration);
    }
  }, [show, duration]);

  if (!shouldRender) return null;

  return (
    <div
      ref={ref}
      className={cn(
        isVisible ? enter : leave,
        isVisible ? (show ? enterTo : enterFrom) : (show ? leaveFrom : leaveTo),
        className
      )}
      {...props}
    >
      {children}
    </div>
  );
});

Transition.displayName = 'Transition';

// Morphing Shape Component
interface MorphProps extends ComponentProps<'div'> {
  shapes?: ('circle' | 'square' | 'diamond' | 'hexagon')[];
  duration?: number;
  autoPlay?: boolean;
}

export const Morph = forwardRef<HTMLDivElement, MorphProps>((
  {
    shapes = ['circle', 'square'],
    duration = 2000,
    autoPlay = true,
    className,
    children,
    ...props
  },
  ref
) => {
  const [currentShape, setCurrentShape] = useState(0);

  useEffect(() => {
    if (!autoPlay) return;

    const interval = setInterval(() => {
      setCurrentShape((prev) => (prev + 1) % shapes.length);
    }, duration);

    return () => clearInterval(interval);
  }, [shapes.length, duration, autoPlay]);

  const shapeClasses = {
    circle: 'rounded-full',
    square: 'rounded-none',
    diamond: 'rotate-45 rounded-sm',
    hexagon: 'clip-path-hexagon',
  };

  return (
    <div
      ref={ref}
      className={cn(
        'transition-all duration-1000 ease-in-out',
        shapeClasses[shapes[currentShape]],
        className
      )}
      {...props}
    >
      {children}
    </div>
  );
});

Morph.displayName = 'Morph';