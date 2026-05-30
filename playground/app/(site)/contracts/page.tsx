import ContractsList from '@/components/ContractsList';

export const metadata = {
  title: 'Validated Contracts — InkPort',
  description: '30 Solidity contracts translated, built, deployed, and asserted on the live Portaldot node.',
};

export default function ContractsPage() {
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
      <ContractsList />
    </div>
  );
}
