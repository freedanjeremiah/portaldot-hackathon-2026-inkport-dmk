import ContractsList from '@/components/ContractsList';

export const metadata = {
  title: 'Validated Contracts — InkPort',
  description: '30 Solidity contracts translated, built, deployed, and asserted on the live Portaldot node.',
};

export default function ContractsPage() {
  return (
    <div className="wrap" style={{ paddingTop: 64, paddingBottom: 80 }}>
      <div style={{ marginBottom: 36 }}>
        <p className="eyebrow">Validated on-chain</p>
        <h1 style={{ fontSize: 46, lineHeight: 1.08, letterSpacing: '-0.03em', fontWeight: 600, margin: '0 0 14px' }}>
          Validated Contracts
        </h1>
        <p style={{ fontSize: 18, color: 'var(--text-dim)', lineHeight: 1.6, maxWidth: 680, margin: 0 }}>
          30 Solidity contracts — each translated, built, deployed, and asserted on the live Portaldot node.
          Real extrinsics, real receipts, no mocks.
        </p>
      </div>
      <ContractsList />
    </div>
  );
}
