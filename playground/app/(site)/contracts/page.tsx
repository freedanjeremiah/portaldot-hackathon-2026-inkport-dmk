import ContractsList from '@/components/ContractsList';

export const metadata = {
  title: 'Validated Contracts — InkPort',
  description: '30 Solidity contracts translated, built, deployed, and asserted on the live Portaldot node.',
};

export default function ContractsPage() {
  return (
    <>
      <section className="page-head grid-bg">
        <div className="wrap">
          <div className="eyebrow">Validated showcase</div>
          <h1>30 contracts, deployed for real.</h1>
          <p className="ph-lead">
            Each contract here was translated, built, deployed, and asserted on the live Portaldot node.
            Real extrinsics, real receipts, no mocks — every checkmark is a contract that actually ran on-chain.
          </p>
        </div>
      </section>
      <div className="wrap" style={{ paddingBottom: 80 }}>
        <ContractsList />
      </div>
    </>
  );
}
