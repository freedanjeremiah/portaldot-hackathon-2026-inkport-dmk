import './site.css';
import './burnt.css';
import SiteNav from '@/components/SiteNav';
import SiteFooter from '@/components/SiteFooter';

export default function SiteLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="site-wrap">
      {/* SVG filter defs for burnt-paper torn edges */}
      <svg className="burn-svg-defs" aria-hidden="true">
        <defs>
          <filter id="burn" x="-5%" y="-5%" width="110%" height="110%">
            <feTurbulence type="fractalNoise" baseFrequency="0.04 0.06" numOctaves="3" seed="2" result="noise"/>
            <feDisplacementMap in="SourceGraphic" in2="noise" scale="10" xChannelSelector="R" yChannelSelector="G"/>
          </filter>
          <filter id="burnFrame" x="-8%" y="-8%" width="116%" height="116%">
            <feTurbulence type="fractalNoise" baseFrequency="0.015 0.025" numOctaves="4" seed="7" result="noise"/>
            <feDisplacementMap in="SourceGraphic" in2="noise" scale="22" xChannelSelector="R" yChannelSelector="G"/>
          </filter>
        </defs>
      </svg>

      {/* Burnt paper material overlays (fixed, pointer-events: none) */}
      <div className="burn-smoke" aria-hidden="true" />
      <div className="burn-grain" aria-hidden="true" />
      <div className="burn-frame" aria-hidden="true" />

      <SiteNav />
      <main style={{ position: 'relative', zIndex: 2 }}>{children}</main>
      <SiteFooter />
    </div>
  );
}
