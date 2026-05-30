export interface DocNavItem {
  label: string;
  href: string;
}
export interface DocNavSection {
  title: string;
  items: DocNavItem[];
}

export const DOCS_NAV: DocNavSection[] = [
  {
    title: 'Getting Started',
    items: [
      { label: 'Install', href: '/docs/getting-started/install' },
      { label: 'First contract', href: '/docs/getting-started/first-contract' },
      { label: 'Project layout', href: '/docs/getting-started/project-layout' },
    ],
  },
  {
    title: 'CLI Reference',
    items: [
      { label: 'translate', href: '/docs/cli/translate' },
      { label: 'build', href: '/docs/cli/build' },
      { label: 'deploy', href: '/docs/cli/deploy' },
      { label: 'call', href: '/docs/cli/call' },
      { label: 'test', href: '/docs/cli/test' },
      { label: 'all', href: '/docs/cli/all' },
    ],
  },
  {
    title: 'Solidity Coverage',
    items: [
      { label: 'Supported surface', href: '/docs/solidity/supported' },
      { label: 'Rejected constructs', href: '/docs/solidity/rejected' },
    ],
  },
  {
    title: 'Guides',
    items: [
      { label: 'ERC20 walkthrough', href: '/docs/guides/erc20' },
      { label: 'Payable contracts', href: '/docs/guides/payable' },
      { label: 'Cross-contract calls', href: '/docs/guides/cross-contract' },
      { label: 'Integer width semantics', href: '/docs/guides/integers' },
    ],
  },
  {
    title: 'Reference',
    items: [
      { label: 'metadata.json format', href: '/docs/reference/metadata' },
      { label: 'Test spec format', href: '/docs/reference/test-spec' },
      { label: 'Portaldot node', href: '/docs/reference/portaldot-node' },
    ],
  },
  {
    title: 'Troubleshooting',
    items: [
      { label: 'Common errors', href: '/docs/troubleshooting' },
    ],
  },
];
