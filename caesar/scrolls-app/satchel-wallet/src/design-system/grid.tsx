// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// @ts-nocheck
/**
 * Caesar Design System - Advanced Grid System
 * Professional responsive grid system for complex data visualization
 */

import React, { forwardRef, ReactNode, ComponentProps } from 'react';
import { cn } from './utils';

// Grid Container Props
interface GridProps extends ComponentProps<'div'> {
  cols?: 1 | 2 | 3 | 4 | 5 | 6 | 8 | 12 | 16 | 24;
  gap?: 'none' | 'xs' | 'sm' | 'md' | 'lg' | 'xl' | '2xl';
  responsive?: {
    sm?: 1 | 2 | 3 | 4 | 5 | 6 | 8 | 12;
    md?: 1 | 2 | 3 | 4 | 5 | 6 | 8 | 12;
    lg?: 1 | 2 | 3 | 4 | 5 | 6 | 8 | 12 | 16;
    xl?: 1 | 2 | 3 | 4 | 5 | 6 | 8 | 12 | 16 | 24;
  };
  autoFit?: boolean;
  minItemWidth?: string;
  maxItemWidth?: string;
}

export const Grid = forwardRef<HTMLDivElement, GridProps>((
  {
    cols = 12,
    gap = 'md',
    responsive,
    autoFit = false,
    minItemWidth = '200px',
    maxItemWidth = '1fr',
    className,
    children,
    style,
    ...props
  },
  ref
) => {
  const gapClasses = {
    none: 'gap-0',
    xs: 'gap-1',
    sm: 'gap-2',
    md: 'gap-4',
    lg: 'gap-6',
    xl: 'gap-8',
    '2xl': 'gap-12',
  };

  const colClasses = {
    1: 'grid-cols-1',
    2: 'grid-cols-2',
    3: 'grid-cols-3',
    4: 'grid-cols-4',
    5: 'grid-cols-5',
    6: 'grid-cols-6',
    8: 'grid-cols-8',
    12: 'grid-cols-12',
    16: 'grid-cols-16',
    24: 'grid-cols-24',
  };

  const responsiveClasses = responsive ? Object.entries(responsive).map(([breakpoint, colCount]) => {
    const breakpointPrefix = breakpoint === 'sm' ? 'sm:' : 
                            breakpoint === 'md' ? 'md:' : 
                            breakpoint === 'lg' ? 'lg:' : 'xl:';
    return `${breakpointPrefix}${colClasses[colCount as keyof typeof colClasses]}`;
  }).join(' ') : '';

  const autoFitStyle = autoFit ? {
    gridTemplateColumns: `repeat(auto-fit, minmax(${minItemWidth}, ${maxItemWidth}))`,
  } : {};

  return (
    <div
      ref={ref}
      className={cn(
        'grid',
        !autoFit && colClasses[cols],
        gapClasses[gap],
        responsiveClasses,
        className
      )}
      style={{
        ...autoFitStyle,
        ...style,
      }}
      {...props}
    >
      {children}
    </div>
  );
});

Grid.displayName = 'Grid';

// Grid Item Props
interface GridItemProps extends ComponentProps<'div'> {
  span?: 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 16 | 24 | 'full';
  spanRow?: 1 | 2 | 3 | 4 | 5 | 6 | 'auto' | 'full';
  start?: 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 16 | 24 | 'auto';
  end?: 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 16 | 24 | 'auto';
  responsive?: {
    sm?: { span?: GridItemProps['span']; start?: GridItemProps['start']; end?: GridItemProps['end'] };
    md?: { span?: GridItemProps['span']; start?: GridItemProps['start']; end?: GridItemProps['end'] };
    lg?: { span?: GridItemProps['span']; start?: GridItemProps['start']; end?: GridItemProps['end'] };
    xl?: { span?: GridItemProps['span']; start?: GridItemProps['start']; end?: GridItemProps['end'] };
  };
}

export const GridItem = forwardRef<HTMLDivElement, GridItemProps>((
  {
    span = 1,
    spanRow = 'auto',
    start = 'auto',
    end = 'auto',
    responsive,
    className,
    children,
    ...props
  },
  ref
) => {
  const spanClasses = {
    1: 'col-span-1',
    2: 'col-span-2',
    3: 'col-span-3',
    4: 'col-span-4',
    5: 'col-span-5',
    6: 'col-span-6',
    7: 'col-span-7',
    8: 'col-span-8',
    9: 'col-span-9',
    10: 'col-span-10',
    11: 'col-span-11',
    12: 'col-span-12',
    16: 'col-span-16',
    24: 'col-span-24',
    full: 'col-span-full',
  };

  const rowSpanClasses = {
    1: 'row-span-1',
    2: 'row-span-2',
    3: 'row-span-3',
    4: 'row-span-4',
    5: 'row-span-5',
    6: 'row-span-6',
    auto: 'row-auto',
    full: 'row-span-full',
  };

  const startClasses = {
    1: 'col-start-1',
    2: 'col-start-2',
    3: 'col-start-3',
    4: 'col-start-4',
    5: 'col-start-5',
    6: 'col-start-6',
    7: 'col-start-7',
    8: 'col-start-8',
    9: 'col-start-9',
    10: 'col-start-10',
    11: 'col-start-11',
    12: 'col-start-12',
    16: 'col-start-16',
    24: 'col-start-24',
    auto: 'col-start-auto',
  };

  const endClasses = {
    1: 'col-end-1',
    2: 'col-end-2',
    3: 'col-end-3',
    4: 'col-end-4',
    5: 'col-end-5',
    6: 'col-end-6',
    7: 'col-end-7',
    8: 'col-end-8',
    9: 'col-end-9',
    10: 'col-end-10',
    11: 'col-end-11',
    12: 'col-end-12',
    16: 'col-end-16',
    24: 'col-end-24',
    auto: 'col-end-auto',
  };

  const responsiveClasses = responsive ? Object.entries(responsive).flatMap(([breakpoint, config]) => {
    const prefix = breakpoint === 'sm' ? 'sm:' : 
                   breakpoint === 'md' ? 'md:' : 
                   breakpoint === 'lg' ? 'lg:' : 'xl:';
    
    const classes = [];
    if (config.span) classes.push(`${prefix}${spanClasses[config.span]}`);
    if (config.start) classes.push(`${prefix}${startClasses[config.start]}`);
    if (config.end) classes.push(`${prefix}${endClasses[config.end]}`);
    return classes;
  }).join(' ') : '';

  return (
    <div
      ref={ref}
      className={cn(
        spanClasses[span],
        rowSpanClasses[spanRow],
        start !== 'auto' && startClasses[start],
        end !== 'auto' && endClasses[end],
        responsiveClasses,
        className
      )}
      {...props}
    >
      {children}
    </div>
  );
});

GridItem.displayName = 'GridItem';

// Flex System for Advanced Layouts
interface FlexProps extends ComponentProps<'div'> {
  direction?: 'row' | 'col' | 'row-reverse' | 'col-reverse';
  wrap?: 'wrap' | 'nowrap' | 'wrap-reverse';
  justify?: 'start' | 'end' | 'center' | 'between' | 'around' | 'evenly';
  align?: 'start' | 'end' | 'center' | 'baseline' | 'stretch';
  gap?: 'none' | 'xs' | 'sm' | 'md' | 'lg' | 'xl' | '2xl';
  responsive?: {
    sm?: Partial<Pick<FlexProps, 'direction' | 'wrap' | 'justify' | 'align'>>;
    md?: Partial<Pick<FlexProps, 'direction' | 'wrap' | 'justify' | 'align'>>;
    lg?: Partial<Pick<FlexProps, 'direction' | 'wrap' | 'justify' | 'align'>>;
    xl?: Partial<Pick<FlexProps, 'direction' | 'wrap' | 'justify' | 'align'>>;
  };
}

export const Flex = forwardRef<HTMLDivElement, FlexProps>((
  {
    direction = 'row',
    wrap = 'nowrap',
    justify = 'start',
    align = 'start',
    gap = 'md',
    responsive,
    className,
    children,
    ...props
  },
  ref
) => {
  const directionClasses = {
    row: 'flex-row',
    col: 'flex-col',
    'row-reverse': 'flex-row-reverse',
    'col-reverse': 'flex-col-reverse',
  };

  const wrapClasses = {
    wrap: 'flex-wrap',
    nowrap: 'flex-nowrap',
    'wrap-reverse': 'flex-wrap-reverse',
  };

  const justifyClasses = {
    start: 'justify-start',
    end: 'justify-end',
    center: 'justify-center',
    between: 'justify-between',
    around: 'justify-around',
    evenly: 'justify-evenly',
  };

  const alignClasses = {
    start: 'items-start',
    end: 'items-end',
    center: 'items-center',
    baseline: 'items-baseline',
    stretch: 'items-stretch',
  };

  const gapClasses = {
    none: 'gap-0',
    xs: 'gap-1',
    sm: 'gap-2',
    md: 'gap-4',
    lg: 'gap-6',
    xl: 'gap-8',
    '2xl': 'gap-12',
  };

  const responsiveClasses = responsive ? Object.entries(responsive).flatMap(([breakpoint, config]) => {
    const prefix = breakpoint === 'sm' ? 'sm:' : 
                   breakpoint === 'md' ? 'md:' : 
                   breakpoint === 'lg' ? 'lg:' : 'xl:';
    
    const classes = [];
    if (config.direction) classes.push(`${prefix}${directionClasses[config.direction]}`);
    if (config.wrap) classes.push(`${prefix}${wrapClasses[config.wrap]}`);
    if (config.justify) classes.push(`${prefix}${justifyClasses[config.justify]}`);
    if (config.align) classes.push(`${prefix}${alignClasses[config.align]}`);
    return classes;
  }).join(' ') : '';

  return (
    <div
      ref={ref}
      className={cn(
        'flex',
        directionClasses[direction],
        wrapClasses[wrap],
        justifyClasses[justify],
        alignClasses[align],
        gapClasses[gap],
        responsiveClasses,
        className
      )}
      {...props}
    >
      {children}
    </div>
  );
});

Flex.displayName = 'Flex';

// Stack Component for Vertical/Horizontal Layouts
interface StackProps extends ComponentProps<'div'> {
  direction?: 'vertical' | 'horizontal';
  spacing?: 'none' | 'xs' | 'sm' | 'md' | 'lg' | 'xl' | '2xl';
  align?: 'start' | 'center' | 'end' | 'stretch';
  divider?: ReactNode;
}

export const Stack = forwardRef<HTMLDivElement, StackProps>((
  {
    direction = 'vertical',
    spacing = 'md',
    align = 'stretch',
    divider,
    className,
    children,
    ...props
  },
  ref
) => {
  const isVertical = direction === 'vertical';
  
  const spacingClasses = {
    none: isVertical ? 'space-y-0' : 'space-x-0',
    xs: isVertical ? 'space-y-1' : 'space-x-1',
    sm: isVertical ? 'space-y-2' : 'space-x-2',
    md: isVertical ? 'space-y-4' : 'space-x-4',
    lg: isVertical ? 'space-y-6' : 'space-x-6',
    xl: isVertical ? 'space-y-8' : 'space-x-8',
    '2xl': isVertical ? 'space-y-12' : 'space-x-12',
  };

  const alignClasses = {
    start: isVertical ? 'items-start' : 'justify-start',
    center: isVertical ? 'items-center' : 'justify-center',
    end: isVertical ? 'items-end' : 'justify-end',
    stretch: isVertical ? 'items-stretch' : 'justify-stretch',
  };

  const childrenArray = React.Children.toArray(children);
  const content = divider && childrenArray.length > 1 
    // @ts-ignore - ReactNode array iteration
    ? childrenArray.reduce((acc, child, index) => {
        if (index === 0) return [child];
        return [...acc, divider, child] as ReactNode[];
      }, [] as ReactNode[])
    : children;

  return (
    <div
      ref={ref}
      className={cn(
        'flex',
        isVertical ? 'flex-col' : 'flex-row',
        spacingClasses[spacing],
        alignClasses[align],
        className
      )}
      {...props}
    >
      {content}
    </div>
  );
});

Stack.displayName = 'Stack';

// Container Component for Max Width and Centering
interface ContainerProps extends ComponentProps<'div'> {
  size?: 'xs' | 'sm' | 'md' | 'lg' | 'xl' | '2xl' | '3xl' | '4xl' | '5xl' | '6xl' | '7xl' | 'full';
  center?: boolean;
  padding?: 'none' | 'sm' | 'md' | 'lg' | 'xl';
}

export const Container = forwardRef<HTMLDivElement, ContainerProps>((
  {
    size = 'xl',
    center = true,
    padding = 'md',
    className,
    children,
    ...props
  },
  ref
) => {
  const sizeClasses = {
    xs: 'max-w-xs',
    sm: 'max-w-sm',
    md: 'max-w-md',
    lg: 'max-w-lg',
    xl: 'max-w-xl',
    '2xl': 'max-w-2xl',
    '3xl': 'max-w-3xl',
    '4xl': 'max-w-4xl',
    '5xl': 'max-w-5xl',
    '6xl': 'max-w-6xl',
    '7xl': 'max-w-7xl',
    full: 'max-w-full',
  };

  const paddingClasses = {
    none: '',
    sm: 'px-4 py-2',
    md: 'px-6 py-4',
    lg: 'px-8 py-6',
    xl: 'px-12 py-8',
  };

  return (
    <div
      ref={ref}
      className={cn(
        sizeClasses[size],
        center && 'mx-auto',
        paddingClasses[padding],
        className
      )}
      {...props}
    >
      {children}
    </div>
  );
});

Container.displayName = 'Container';

// Aspect Ratio Component
interface AspectRatioProps extends ComponentProps<'div'> {
  ratio?: '1/1' | '4/3' | '16/9' | '21/9' | '3/2' | '2/3';
}

export const AspectRatio = forwardRef<HTMLDivElement, AspectRatioProps>((
  {
    ratio = '16/9',
    className,
    children,
    ...props
  },
  ref
) => {
  const ratioClasses = {
    '1/1': 'aspect-square',
    '4/3': 'aspect-[4/3]',
    '16/9': 'aspect-video',
    '21/9': 'aspect-[21/9]',
    '3/2': 'aspect-[3/2]',
    '2/3': 'aspect-[2/3]',
  };

  return (
    <div
      ref={ref}
      className={cn(
        ratioClasses[ratio],
        className
      )}
      {...props}
    >
      {children}
    </div>
  );
});

AspectRatio.displayName = 'AspectRatio';