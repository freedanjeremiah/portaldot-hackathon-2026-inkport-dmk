export default function SiteFooter() {
  return (
    <footer className="footer">
      <div className="footer-inner">
        <div className="footer-brand">
          <span className="nav-mark" style={{ width: 22, height: 22 }}>
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7">
              <path d="M13 2L4 14h7l-1 8 9-12h-7l1-8z" strokeLinejoin="round"/>
            </svg>
          </span>
          <span>Ink<b style={{ color: 'var(--accent-3)' }}>Port</b></span>
        </div>
        <div className="footer-links">
          <span className="footer-sep">·</span>
          <a href="https://opensource.org/license/mit" target="_blank" rel="noopener noreferrer">MIT License</a>
          <span className="footer-sep">·</span>
          <a href="https://github.com/freedanjeremiah/inkide" target="_blank" rel="noopener noreferrer">GitHub</a>
        </div>
        <div className="footer-spacer" />
        <div className="footer-node">Portaldot: wss://portaldot.philotheephilix.in</div>
      </div>
    </footer>
  );
}
