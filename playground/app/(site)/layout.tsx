import './site.css';
import SiteNav from '@/components/SiteNav';
import SiteFooter from '@/components/SiteFooter';

export default function SiteLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="site-shell">
      <SiteNav />
      <main style={{ flex: 1 }}>{children}</main>
      <SiteFooter />
    </div>
  );
}
