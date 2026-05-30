import DocsSidebar from '@/components/DocsSidebar';

export default function DocsLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="docs-shell">
      <DocsSidebar />
      <div className="docs-content">{children}</div>
    </div>
  );
}
