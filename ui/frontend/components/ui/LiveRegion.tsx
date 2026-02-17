// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { useEffect, useState } from 'react';

interface LiveRegionProps {
  message: string;
  politeness?: 'polite' | 'assertive';
  clearOnUpdate?: boolean;
  className?: string;
}

export function LiveRegion({ 
  message, 
  politeness = 'polite', 
  clearOnUpdate = true,
  className = ''
}: LiveRegionProps) {
  const [announcements, setAnnouncements] = useState<string[]>([]);

  useEffect(() => {
    if (message) {
      setAnnouncements(prev => clearOnUpdate ? [message] : [...prev, message]);
      
      if (clearOnUpdate) {
        const timer = setTimeout(() => {
          setAnnouncements([]);
        }, 1000);
        return () => clearTimeout(timer);
      }
    }
  }, [message, clearOnUpdate]);

  return (
    <div
      aria-live={politeness}
      aria-atomic="true"
      className={`sr-only ${className}`}
    >
      {announcements.map((announcement, index) => (
        <div key={index}>{announcement}</div>
      ))}
    </div>
  );
}
