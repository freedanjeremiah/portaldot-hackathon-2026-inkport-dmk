'use client';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { DOCS_NAV } from '@/lib/docs-nav';

export default function DocsSidebar() {
  const path = usePathname();
  return (
    <aside className="docs-sidebar">
      {DOCS_NAV.map(section => (
        <div key={section.title} className="docs-sidebar-section">
          <div className="docs-sidebar-section-title">{section.title}</div>
          {section.items.map(item => (
            <Link
              key={item.href}
              href={item.href}
              className={`docs-sidebar-link${path === item.href ? ' active' : ''}`}
            >
              {item.label}
            </Link>
          ))}
        </div>
      ))}
    </aside>
  );
}
