'use client';
import Link from 'next/link';
import { usePathname } from 'next/navigation';

export default function SiteNav() {
  const path = usePathname();

  return (
    <header className="nav">
      <div className="nav-inner">
        <Link href="/home" className="nav-brand">
          <span className="nav-mark">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7">
              <path d="M13 2L4 14h7l-1 8 9-12h-7l1-8z" strokeLinejoin="round"/>
            </svg>
          </span>
          <span>Ink<b>Port</b></span>
        </Link>
        <nav className="nav-links">
          <Link href="/docs/getting-started/install" className={`nav-link${path.startsWith('/docs') ? ' active' : ''}`}>Docs</Link>
          <Link href="/contracts" className={`nav-link${path === '/contracts' ? ' active' : ''}`}>Contracts</Link>
          <Link href="/why-inkport" className={`nav-link${path === '/why-inkport' ? ' active' : ''}`}>Why InkPort</Link>
        </nav>
        <div className="nav-spacer" />
        <a
          href="https://github.com/freedanjeremiah/inkide"
          className="nav-ghost"
          target="_blank"
          rel="noopener noreferrer"
        >
          <svg width="15" height="15" viewBox="0 0 24 24" fill="currentColor">
            <path d="M12 .5C5.7.5.5 5.7.5 12c0 5.1 3.3 9.4 7.9 10.9.6.1.8-.2.8-.5v-2c-3.2.7-3.9-1.4-3.9-1.4-.5-1.3-1.3-1.7-1.3-1.7-1-.7.1-.7.1-.7 1.2.1 1.8 1.2 1.8 1.2 1 1.8 2.7 1.3 3.4 1 .1-.8.4-1.3.7-1.6-2.6-.3-5.3-1.3-5.3-5.8 0-1.3.5-2.3 1.2-3.1-.1-.3-.5-1.5.1-3.1 0 0 1-.3 3.3 1.2a11.5 11.5 0 016 0C17.3 4.6 18.3 5 18.3 5c.6 1.6.2 2.8.1 3.1.8.8 1.2 1.8 1.2 3.1 0 4.5-2.7 5.5-5.3 5.8.4.4.8 1.1.8 2.2v3.3c0 .3.2.7.8.5A11.5 11.5 0 0023.5 12C23.5 5.7 18.3.5 12 .5z"/>
          </svg>
          <span>GitHub</span>
        </a>
        <Link href="/" className="nav-cta">
          Try Playground
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M5 12h14M13 6l6 6-6 6" strokeLinecap="round" strokeLinejoin="round"/>
          </svg>
        </Link>
      </div>
    </header>
  );
}
