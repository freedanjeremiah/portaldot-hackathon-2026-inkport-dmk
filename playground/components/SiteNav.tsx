'use client';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { Ic } from '@/components/Pipeline';

export default function SiteNav() {
  const path = usePathname();

  return (
    <nav className="site-nav">
      <Link href="/home" className="site-brand">
        <span className="site-brand-mark">
          <Ic.bolt width={13} height={13} />
        </span>
        Ink<strong>Port</strong>
      </Link>
      <div className="site-nav-links">
        <Link href="/docs/getting-started/install" className={path.startsWith('/docs') ? 'active' : ''}>Docs</Link>
        <Link href="/contracts" className={path === '/contracts' ? 'active' : ''}>Contracts</Link>
        <Link href="/why-inkport" className={path === '/why-inkport' ? 'active' : ''}>Why InkPort</Link>
      </div>
      <div className="site-nav-spacer" />
      <Link href="/" className="btn btn-primary site-nav-cta">Try Playground →</Link>
    </nav>
  );
}
