// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import React, { ReactNode } from 'react';
import { Sidebar } from './Sidebar';
import { Header } from './Header';
import { UpdateBanner } from './UpdateBanner';

interface LayoutProps {
  children: ReactNode;
}

export function Layout({ children }: LayoutProps) {
  return (
    <div className="flex h-screen bg-gradient-to-br from-black via-slate-900 to-black">
      <Sidebar />
      <div className="flex-1 flex flex-col overflow-hidden">
        {/* Phase J.1 — surfaces foundation-published release feed entries.
            Renders as a no-op when the daemon endpoint returns 404. */}
        <UpdateBanner />
        <Header />
        <main className="flex-1 overflow-auto p-6 bg-gradient-to-b from-transparent to-black/20">
          {children}
        </main>
      </div>
    </div>
  );
}
