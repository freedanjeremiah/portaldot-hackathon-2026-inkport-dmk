'use client';
import { useState } from 'react';
import ContractCard from '@/components/ContractCard';
import { CONTRACTS, ALL_TAGS, type ContractTag } from '@/lib/contracts-data';

export default function ContractsList() {
  const [active, setActive] = useState<ContractTag | null>(null);
  const filtered = active ? CONTRACTS.filter(c => c.tags.includes(active)) : CONTRACTS;

  return (
    <>
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
    </>
  );
}
