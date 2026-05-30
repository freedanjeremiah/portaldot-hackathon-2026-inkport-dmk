'use client';
import { useState } from 'react';
import { CONTRACTS, ALL_TAGS, type ContractTag, type ContractEntry } from '@/lib/contracts-data';

const TAG_CLASS: Record<ContractTag, string> = {
  basics: 'tag-basics', math: 'tag-math', tokens: 'tag-tokens',
  payable: 'tag-payable', access: 'tag-access', 'cross-contract': 'tag-cross-contract',
  oop: 'tag-oop', strings: 'tag-strings', arrays: 'tag-arrays',
};

function TemplateCard({ c }: { c: ContractEntry }) {
  return (
    <a href={`/playground/editor?contract=${encodeURIComponent(c.name)}`} className="tpl-card">
      <div className="contract-card-head">
        <span className="contract-name">{c.name}</span>
        {c.tags.map(t => (
          <span key={t} className={`contract-tag ${TAG_CLASS[t]}`}>{t}</span>
        ))}
      </div>
      <p className="contract-desc">{c.description}</p>
      <p className="contract-sig">{c.constructor}</p>
      <p className="contract-sig" style={{ color: 'var(--text-faint)' }}>
        {c.messages.slice(0, 3).join(' · ')}{c.messages.length > 3 ? ' …' : ''}
      </p>
      <div className="tpl-card-cta">
        <span className="tpl-open">Open in editor</span>
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M5 12h14M13 6l6 6-6 6"/>
        </svg>
      </div>
    </a>
  );
}

export default function TemplateGallery() {
  const [active, setActive] = useState<ContractTag | null>(null);
  const [query, setQuery] = useState('');

  const q = query.trim().toLowerCase();
  const filtered = CONTRACTS.filter(c => {
    if (active && !c.tags.includes(active)) return false;
    if (!q) return true;
    return (
      c.name.toLowerCase().includes(q) ||
      c.description.toLowerCase().includes(q) ||
      c.messages.some(m => m.toLowerCase().includes(q)) ||
      c.tags.some(t => t.toLowerCase().includes(q))
    );
  });

  return (
    <>
      <div className="tpl-controls">
        <input
          className="tpl-search"
          type="text"
          placeholder="Search templates — name, message, tag…"
          value={query}
          onChange={e => setQuery(e.target.value)}
        />
        <div className="contracts-filter">
          <button
            className={`filter-btn${active === null ? ' active' : ''}`}
            onClick={() => setActive(null)}
          >
            All ({CONTRACTS.length})
          </button>
          {ALL_TAGS.map(tag => {
            const count = CONTRACTS.filter(c => c.tags.includes(tag)).length;
            return (
              <button
                key={tag}
                className={`filter-btn${active === tag ? ' active' : ''}`}
                onClick={() => setActive(active === tag ? null : tag)}
              >
                {tag} ({count})
              </button>
            );
          })}
        </div>
      </div>

      <div className="contracts-grid">
        {/* Custom contract — always first, prominent */}
        <a href="/playground/editor?contract=custom" className="tpl-card tpl-card-custom">
          <div className="contract-card-head">
            <span className="contract-name">Custom contract</span>
            <span className="contract-tag tag-tokens">blank</span>
          </div>
          <p className="contract-desc">
            Start from a minimal starter and write or paste your own Solidity.
            Translate → compile → deploy → call against the live node.
          </p>
          <p className="contract-sig">contract MyContract &#123; … &#125;</p>
          <div className="tpl-card-cta">
            <span className="tpl-open">New contract</span>
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M12 5v14M5 12h14"/>
            </svg>
          </div>
        </a>

        {filtered.map(c => <TemplateCard key={c.name} c={c} />)}
      </div>

      {filtered.length === 0 && (
        <p style={{ color: 'var(--text-dim)', fontFamily: 'var(--mono)', fontSize: 14, marginTop: 24 }}>
          No templates match “{query}”.
        </p>
      )}
    </>
  );
}
