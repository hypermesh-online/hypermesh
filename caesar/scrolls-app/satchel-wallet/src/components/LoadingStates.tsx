// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React from 'react';
import { Loader2, Wallet, ArrowUpDown } from 'lucide-react';

interface LoadingSpinnerProps {
  size?: 'sm' | 'md' | 'lg';
  className?: string;
}

export const LoadingSpinner: React.FC<LoadingSpinnerProps> = ({ 
  size = 'md', 
  className = '' 
}) => {
  const sizeClasses = {
    sm: 'w-4 h-4',
    md: 'w-6 h-6', 
    lg: 'w-8 h-8'
  };

  return (
    <Loader2 
      className={`animate-spin text-blue-600 ${sizeClasses[size]} ${className}`} 
    />
  );
};

interface LoadingButtonProps {
  children: React.ReactNode;
  isLoading: boolean;
  disabled?: boolean;
  onClick?: () => void;
  variant?: 'primary' | 'secondary';
  className?: string;
}

export const LoadingButton: React.FC<LoadingButtonProps> = ({
  children,
  isLoading,
  disabled = false,
  onClick,
  variant = 'primary',
  className = ''
}) => {
  const baseClasses = "px-4 py-2 rounded-lg font-medium transition-colors flex items-center justify-center gap-2 min-w-[120px]";
  
  const variantClasses = {
    primary: "bg-blue-600 text-white hover:bg-blue-700 disabled:bg-blue-300 disabled:cursor-not-allowed",
    secondary: "bg-gray-100 text-gray-900 hover:bg-gray-200 disabled:bg-gray-50 disabled:cursor-not-allowed"
  };

  return (
    <button
      onClick={onClick}
      disabled={disabled || isLoading}
      className={`${baseClasses} ${variantClasses[variant]} ${className}`}
    >
      {isLoading ? (
        <>
          <LoadingSpinner size="sm" className="text-current" />
          <span>Loading...</span>
        </>
      ) : (
        children
      )}
    </button>
  );
};

interface LoadingCardProps {
  title?: string;
  message?: string;
  icon?: React.ReactNode;
}

export const LoadingCard: React.FC<LoadingCardProps> = ({ 
  title = "Loading",
  message = "Please wait while we fetch your data",
  icon = <Wallet className="text-blue-600" size={24} />
}) => {
  return (
    <div className="bg-white border border-gray-200 rounded-lg p-6 text-center">
      <div className="flex flex-col items-center space-y-4">
        <div className="flex items-center justify-center w-12 h-12 bg-blue-50 rounded-full">
          {icon}
        </div>
        <div className="space-y-2">
          <h3 className="font-semibold text-gray-900">{title}</h3>
          <p className="text-sm text-gray-500">{message}</p>
        </div>
        <LoadingSpinner size="md" />
      </div>
    </div>
  );
};

interface SkeletonProps {
  className?: string;
  variant?: 'text' | 'rectangular' | 'circular';
}

export const Skeleton: React.FC<SkeletonProps> = ({ 
  className = '',
  variant = 'rectangular'
}) => {
  const variantClasses = {
    text: 'h-4 rounded',
    rectangular: 'h-12 rounded-lg',
    circular: 'rounded-full aspect-square'
  };

  return (
    <div 
      className={`bg-gray-200 animate-pulse ${variantClasses[variant]} ${className}`}
    />
  );
};

export const WalletLoadingSkeleton: React.FC = () => {
  return (
    <div className="px-4 py-6 max-w-md mx-auto space-y-6">
      {/* Balance Section Skeleton */}
      <div className="text-center py-8">
        <Skeleton variant="text" className="w-24 h-3 mx-auto mb-2" />
        <Skeleton variant="text" className="w-32 h-8 mx-auto mb-6" />
        
        {/* Action Buttons Skeleton */}
        <div className="flex gap-3 justify-center">
          <Skeleton className="w-16 h-10" />
          <Skeleton className="w-20 h-10" />
          <Skeleton className="w-14 h-10" />
        </div>
      </div>

      {/* Assets Section Skeleton */}
      <div className="bg-gray-50 rounded-xl p-4">
        <Skeleton variant="text" className="w-16 h-5 mb-4" />
        <div className="space-y-3">
          {[1, 2].map((i) => (
            <div key={i} className="flex items-center justify-between p-3 bg-white rounded-lg">
              <div className="flex items-center gap-3">
                <Skeleton variant="circular" className="w-10 h-10" />
                <div className="space-y-2">
                  <Skeleton variant="text" className="w-20 h-4" />
                  <Skeleton variant="text" className="w-16 h-3" />
                </div>
              </div>
              <div className="text-right space-y-2">
                <Skeleton variant="text" className="w-20 h-4" />
                <Skeleton variant="text" className="w-12 h-3" />
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};

interface TransactionLoadingProps {
  message?: string;
}

export const TransactionLoading: React.FC<TransactionLoadingProps> = ({ 
  message = "Processing transaction..." 
}) => {
  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
      <div className="bg-white rounded-lg shadow-xl max-w-sm w-full p-6">
        <div className="text-center space-y-4">
          <div className="flex justify-center">
            <div className="relative">
              <div className="w-16 h-16 bg-blue-50 rounded-full flex items-center justify-center">
                <ArrowUpDown className="text-blue-600" size={24} />
              </div>
              <div className="absolute inset-0 border-4 border-blue-600 border-t-transparent rounded-full animate-spin"></div>
            </div>
          </div>
          <div>
            <h3 className="font-semibold text-gray-900 mb-2">Transaction Processing</h3>
            <p className="text-sm text-gray-500">{message}</p>
            <p className="text-xs text-gray-400 mt-2">This may take a few moments</p>
          </div>
        </div>
      </div>
    </div>
  );
};