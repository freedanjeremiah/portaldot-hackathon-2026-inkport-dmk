'use client';
import { useState } from 'react';
import ContractCard from '@/components/ContractCard';
import { CONTRACTS, ALL_TAGS, type ContractTag } from '@/lib/contracts-data';

export default function ContractsPage() {
  const [active, setActive] = useState<ContractTag | null>(null);

  const filtered = active ? CONTRACTS.filter(c => c.tags.includes(active)) : CONTRACTS;

  return (
    <div className="site-container" style={{ paddingTop: 48, paddingBottom: 80 }}>
      <div style={{ marginBottom: 36 }}>
        <h1 className="site-h1" style={{ textAlign: 'left', fontSize: 'var(--h2)', marginBottom: 10 }}>
          Validated Contracts
        </h1>
        <p style={{ fontSize: 'var(--body)', color: 'var(--text-dim)', lineHeight: 1.65, maxWidth: 580, margin: 0 }}>
          30 Solidity contracts — each translated, built, deployed, and asserted on the live Portaldot node.
          Real extrinsics, real receipts, no mocks.
        </p>
      </div>

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

      <div className="contracts-grid">
        {filtered.map(c => <ContractCard key={c.name} c={c} />)}
      </div>
    </div>
  );
}
