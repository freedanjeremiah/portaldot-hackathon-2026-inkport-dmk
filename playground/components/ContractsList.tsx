'use client';
import { useState } from 'react';
import ContractCard from '@/components/ContractCard';
import { CONTRACTS, ALL_TAGS, type ContractTag } from '@/lib/contracts-data';

const LABELS: Record<ContractTag, string> = {
  basics: 'Basics', math: 'Math', tokens: 'Tokens', payable: 'Payable',
  access: 'Access', 'cross-contract': 'Cross-contract', oop: 'OOP',
  strings: 'Strings', arrays: 'Arrays',
};

export default function ContractsList() {
  const [active, setActive] = useState<ContractTag | null>(null);
  const filtered = active ? CONTRACTS.filter(c => c.tags.includes(active)) : CONTRACTS;

  return (
    <>
      <div className="filter-bar">
        <button
          className={`filter-chip${active === null ? ' active' : ''}`}
          onClick={() => setActive(null)}
        >
          All <span className="fc-count">{CONTRACTS.length}</span>
        </button>
        {ALL_TAGS.map(tag => {
          const count = CONTRACTS.filter(c => c.tags.includes(tag)).length;
          return (
            <button
              key={tag}
              className={`filter-chip${active === tag ? ' active' : ''}`}
              onClick={() => setActive(active === tag ? null : tag)}
            >
              {LABELS[tag]} <span className="fc-count">{count}</span>
            </button>
          );
        })}
      </div>
      <div className="ct-grid">
        {filtered.map(c => <ContractCard key={c.name} c={c} />)}
      </div>
    </>
  );
}
