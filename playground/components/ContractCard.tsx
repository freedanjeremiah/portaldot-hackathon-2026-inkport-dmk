import type { ContractEntry, ContractTag } from '@/lib/contracts-data';

const TAG_CLASS: Record<ContractTag, string> = {
  basics: 'tag-basics', math: 'tag-math', tokens: 'tag-tokens',
  payable: 'tag-payable', access: 'tag-access', 'cross-contract': 'tag-cross-contract',
  oop: 'tag-oop', strings: 'tag-strings', arrays: 'tag-arrays',
};

const CheckIcon = () => (
  <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4">
    <path d="M5 12l5 5L19 7" strokeLinecap="round" strokeLinejoin="round"/>
  </svg>
);
const BeakerIcon = () => (
  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8">
    <path d="M9 3h6M10 3v6l-5 9a2 2 0 002 3h10a2 2 0 002-3l-5-9V3" strokeLinecap="round" strokeLinejoin="round"/>
  </svg>
);
const FileIcon = () => (
  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8">
    <path d="M14 3H7a2 2 0 00-2 2v14a2 2 0 002 2h10a2 2 0 002-2V8l-5-5z" strokeLinecap="round" strokeLinejoin="round"/>
    <path d="M14 3v5h5"/>
  </svg>
);

export default function ContractCard({ c }: { c: ContractEntry }) {
  return (
    <div className="ct-card">
      <div className="ct-top">
        <div className="ct-titlerow">
          <span className="ct-name">{c.name}</span>
          <span className="ct-check"><CheckIcon />deployed</span>
        </div>
        <div className="ct-tags">
          {c.tags.map(t => (
            <span key={t} className={`tag ${TAG_CLASS[t]}`}>{t}</span>
          ))}
        </div>
        <p className="ct-desc">{c.description}</p>
      </div>
      <div className="ct-meta">
        <div className="ct-sig">{c.constructor}</div>
        <div className="ct-msgs">
          {c.messages.slice(0, 4).map((m, i) => (
            <span key={i} className="ct-msg">{m.split('(')[0]}</span>
          ))}
          {c.messages.length > 4 && (
            <span className="ct-msg">+{c.messages.length - 4}</span>
          )}
        </div>
      </div>
      <div className="ct-foot">
        <span className="ct-steps">
          <BeakerIcon /> <b>{c.testSteps}</b> test steps
        </span>
        <span className="ct-links">
          <a className="ct-link" href={c.solFile} target="_blank" rel="noopener noreferrer">
            <FileIcon /> .sol
          </a>
          <a className="ct-link" href={c.testFile} target="_blank" rel="noopener noreferrer">
            <BeakerIcon /> tests
          </a>
        </span>
      </div>
    </div>
  );
}
