import type { ContractEntry, ContractTag } from '@/lib/contracts-data';

const TAG_CLASS: Record<ContractTag, string> = {
  basics: 'tag-basics', math: 'tag-math', tokens: 'tag-tokens',
  payable: 'tag-payable', access: 'tag-access', 'cross-contract': 'tag-cross-contract',
  oop: 'tag-oop', strings: 'tag-strings', arrays: 'tag-arrays',
};

export default function ContractCard({ c }: { c: ContractEntry }) {
  return (
    <div className="contract-card">
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
      <div className="contract-footer">
        <span className="contract-status">
          <span style={{ width: 6, height: 6, borderRadius: '50%', background: 'var(--green)', display: 'inline-block' }} />
          deployed · {c.testSteps} test steps
        </span>
        <div className="contract-links">
          <a href={c.solFile} className="contract-link" target="_blank" rel="noopener noreferrer">.sol</a>
          <a href={c.testFile} className="contract-link" target="_blank" rel="noopener noreferrer">test</a>
        </div>
      </div>
    </div>
  );
}
