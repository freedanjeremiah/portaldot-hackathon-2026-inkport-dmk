export default function SiteFooter() {
  return (
    <footer className="footer">
      <div className="footer-inner">
        <div className="footer-brand">
          <span className="nav-mark" style={{ width: 22, height: 22 }}>
            <img src="/inkport.png" alt="InkPort" style={{ width: '100%', height: '100%', objectFit: 'cover', borderRadius: 'inherit' }} />
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
