import '../(site)/site.css';
import '../(site)/burnt.css';
import './gallery.css';
import TemplateGallery from '@/components/TemplateGallery';

export const metadata = {
  title: 'Playground — Pick a Template — InkPort',
  description: 'Choose a Solidity contract to translate, compile, deploy, and call on the live Portaldot node — or start a custom contract.',
};

export default function PlaygroundGalleryPage() {
  return (
    <div className="tpl-page">
      {/* SVG filter defs for burnt-paper torn edges */}
      <svg className="burn-svg-defs" aria-hidden="true">
        <defs>
          <filter id="burn" x="-5%" y="-5%" width="110%" height="110%">
            <feTurbulence type="fractalNoise" baseFrequency="0.04 0.06" numOctaves="3" seed="2" result="noise"/>
            <feDisplacementMap in="SourceGraphic" in2="noise" scale="10" xChannelSelector="R" yChannelSelector="G"/>
          </filter>
        </defs>
      </svg>

      {/* Burnt paper material overlays (fixed, pointer-events: none) */}
      <div className="burn-smoke" aria-hidden="true" />
      <div className="burn-grain" aria-hidden="true" />
      <div className="burn-frame" aria-hidden="true" />

      <div className="tpl-hero">
        <a href="/" className="tpl-back">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M15 6l-6 6 6 6"/>
          </svg>
          InkPort
        </a>
        <p className="eyebrow">Playground</p>
        <h1>Pick a template</h1>
        <p className="tpl-lede">
          Choose a Solidity contract to load into the editor — translate to seal0 Rust,
          compile to wasm, then deploy and call it on the live Portaldot node. Or start
          a custom contract and write your own.
        </p>
      </div>

      <div className="tpl-body">
        <TemplateGallery />
      </div>
    </div>
  );
}
