# InkPort Website Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> **UI/UX:** Invoke `ui-ux-pro-max` skill before implementing any visual component (Task 2 onwards).
> **Worktrees:** Tasks 4–6 and 8–12 are fully independent — run in parallel worktrees after Task 3 merges.
> **Model:** Use Opus 4.8 for Tasks 2, 4, 5, 6, 7 (visual/content-heavy).

**Goal:** Build the InkPort marketing site (landing, docs, contracts, why-inkport) as new routes alongside the existing playground without touching `app/page.tsx`, `app/playground/`, or `app/api/`.

**Architecture:** Next.js App Router route group `app/(site)/` provides shared nav+footer layout for all marketing pages. MDX powers docs content. All new files are additive — zero changes to existing playground files.

**Tech Stack:** Next.js 15 App Router, `@next/mdx`, plain CSS extending existing design tokens (IBM Plex Sans + JetBrains Mono already loaded), TypeScript.

**Route map (all new):**
```
/home                              → app/(site)/home/page.tsx
/why-inkport                       → app/(site)/why-inkport/page.tsx
/contracts                         → app/(site)/contracts/page.tsx
/docs                              → app/(site)/docs/page.tsx  (redirects to install)
/docs/getting-started/install      → app/(site)/docs/getting-started/install/page.mdx
/docs/getting-started/first-contract
/docs/getting-started/project-layout
/docs/cli/[translate|build|deploy|call|test|all]
/docs/solidity/[supported|rejected]
/docs/guides/[erc20|payable|cross-contract|integers]
/docs/reference/[metadata|test-spec|portaldot-node]
/docs/troubleshooting
```

**Parallel execution waves:**
- Wave 1 (sequential): Task 1 → Task 2 → Task 3
- Wave 2 (parallel worktrees): Tasks 4, 5, 6, 7 simultaneously
- Wave 3 (parallel worktrees): Tasks 8, 9, 10, 11, 12 simultaneously

---

## Task 1: MDX setup + design token CSS

**Files:**
- Modify: `playground/next.config.ts`
- Create: `playground/mdx-components.tsx`
- Create: `playground/app/(site)/site.css`

- [ ] **Step 1: Install MDX packages**

```bash
cd playground
npm install @next/mdx @mdx-js/loader @mdx-js/react
npm install -D @types/mdx
```

Expected: clean install, no peer-dep errors.

- [ ] **Step 2: Update next.config.ts**

```typescript
import createMDX from '@next/mdx';
import type { NextConfig } from 'next';

const withMDX = createMDX({});

const nextConfig: NextConfig = {
  pageExtensions: ['js', 'jsx', 'ts', 'tsx', 'md', 'mdx'],
};

export default withMDX(nextConfig);
```

- [ ] **Step 3: Create mdx-components.tsx (required by @next/mdx)**

`playground/mdx-components.tsx`:
```typescript
import type { MDXComponents } from 'mdx/types';

export function useMDXComponents(components: MDXComponents): MDXComponents {
  return { ...components };
}
```

- [ ] **Step 4: Create site.css with marketing design tokens**

`playground/app/(site)/site.css`:
```css
/* ── Marketing site design tokens (extends playground globals.css) ── */
:root {
  --site-max: 1120px;
  --site-pad: clamp(16px, 4vw, 48px);

  --h1: clamp(2rem, 5vw, 3.5rem);
  --h2: clamp(1.4rem, 3vw, 2rem);
  --h3: 1.2rem;
  --body: 0.9375rem;
  --small: 0.8125rem;
  --lh: 1.65;

  --radius: 10px;
  --radius-sm: 6px;

  --gradient-hero: linear-gradient(135deg, #3b82f6 0%, #8b5cf6 50%, #10b981 100%);
  --gradient-card: linear-gradient(135deg, rgba(59,130,246,0.06), rgba(139,92,246,0.04));
}

/* ── Site shell ── */
.site-shell { display: flex; flex-direction: column; min-height: 100vh; background: var(--bg); }
.site-container { width: 100%; max-width: var(--site-max); margin: 0 auto; padding: 0 var(--site-pad); }

/* ── Nav ── */
.site-nav {
  position: sticky; top: 0; z-index: 100;
  height: 60px; display: flex; align-items: center; gap: 32px;
  padding: 0 var(--site-pad);
  background: rgba(15,15,15,0.85); backdrop-filter: blur(12px);
  border-bottom: 1px solid var(--border);
}
.site-brand { display: flex; align-items: center; gap: 8px; text-decoration: none; color: var(--text); font-size: 15px; font-weight: 600; letter-spacing: -0.01em; }
.site-brand-mark { width: 26px; height: 26px; border-radius: 7px; background: linear-gradient(135deg, var(--accent), #1d4ed8); display: grid; place-items: center; font-size: 13px; box-shadow: 0 0 0 1px rgba(255,255,255,0.08) inset; }
.site-brand strong { color: var(--accent-2); }
.site-nav-links { display: flex; align-items: center; gap: 4px; margin-left: 8px; }
.site-nav-links a { font-size: var(--small); font-weight: 500; color: var(--text-dim); padding: 6px 12px; border-radius: var(--radius-sm); text-decoration: none; transition: color 0.12s, background 0.12s; }
.site-nav-links a:hover, .site-nav-links a.active { color: var(--text); background: rgba(255,255,255,0.05); }
.site-nav-spacer { flex: 1; }
.site-nav-cta { font-size: var(--small) !important; padding: 7px 14px !important; white-space: nowrap; }

/* ── Footer ── */
.site-footer { display: flex; align-items: center; justify-content: center; gap: 14px; height: 52px; border-top: 1px solid var(--border); font-family: var(--mono); font-size: 11px; color: var(--text-faint); flex-shrink: 0; }
.site-footer a { color: var(--text-dim); text-decoration: none; }
.site-footer a:hover { color: var(--text); }

/* ── Page hero ── */
.site-hero { padding: clamp(64px, 10vw, 120px) 0 clamp(48px, 7vw, 80px); text-align: center; }
.site-hero-eyebrow { font-family: var(--mono); font-size: 11px; text-transform: uppercase; letter-spacing: 0.1em; color: var(--accent-2); margin-bottom: 20px; display: inline-flex; align-items: center; gap: 8px; }
.site-hero-eyebrow::before, .site-hero-eyebrow::after { content: ''; display: block; width: 24px; height: 1px; background: var(--accent-2); opacity: 0.5; }
.site-h1 { font-size: var(--h1); font-weight: 700; line-height: 1.1; letter-spacing: -0.03em; color: var(--text); margin-bottom: 20px; }
.site-h1 .gradient-text { background: var(--gradient-hero); -webkit-background-clip: text; -webkit-text-fill-color: transparent; background-clip: text; }
.site-hero-sub { font-size: clamp(0.95rem, 2vw, 1.125rem); color: var(--text-dim); line-height: var(--lh); max-width: 580px; margin: 0 auto 36px; }
.site-hero-actions { display: flex; gap: 12px; justify-content: center; flex-wrap: wrap; }

/* ── Pipeline strip ── */
.pipeline-strip { display: flex; align-items: center; gap: 0; overflow-x: auto; padding: 4px 0; }
.pipeline-step { display: flex; flex-direction: column; align-items: center; gap: 6px; min-width: 120px; padding: 16px; background: var(--panel); border: 1px solid var(--border); border-radius: var(--radius); }
.pipeline-step-label { font-size: 12px; font-weight: 600; color: var(--text); }
.pipeline-step-sub { font-family: var(--mono); font-size: 10px; color: var(--text-faint); text-align: center; line-height: 1.4; }
.pipeline-step-cmd { font-family: var(--mono); font-size: 10px; color: var(--accent-2); margin-top: 4px; }
.pipeline-arrow { color: var(--text-faint); font-size: 16px; padding: 0 8px; flex-shrink: 0; }

/* ── Stats bar ── */
.stats-bar { display: flex; gap: 0; border: 1px solid var(--border); border-radius: var(--radius); background: var(--panel); overflow: hidden; }
.stat-item { flex: 1; padding: 20px 24px; text-align: center; border-right: 1px solid var(--border); }
.stat-item:last-child { border-right: none; }
.stat-num { font-size: clamp(1.4rem, 3vw, 2rem); font-weight: 700; color: var(--text); letter-spacing: -0.02em; font-family: var(--mono); }
.stat-label { font-size: var(--small); color: var(--text-dim); margin-top: 3px; }

/* ── Two-column value prop ── */
.value-cols { display: grid; grid-template-columns: 1fr 1fr; gap: 24px; }
@media (max-width: 640px) { .value-cols { grid-template-columns: 1fr; } }
.value-col { padding: 28px; background: var(--panel); border: 1px solid var(--border); border-radius: var(--radius); background: var(--gradient-card); }
.value-col-title { font-size: 13px; font-weight: 600; color: var(--text); margin-bottom: 10px; display: flex; align-items: center; gap: 8px; }
.value-col-title .vc-dot { width: 8px; height: 8px; border-radius: 50%; background: var(--accent); }
.value-col p { font-size: var(--body); color: var(--text-dim); line-height: var(--lh); margin: 0; }

/* ── Quick install ── */
.install-block { background: var(--panel-3); border: 1px solid var(--border); border-radius: var(--radius); overflow: hidden; }
.install-block-header { display: flex; align-items: center; gap: 8px; padding: 10px 16px; background: var(--panel); border-bottom: 1px solid var(--border); font-family: var(--mono); font-size: 11px; color: var(--text-faint); }
.install-block-header .dot { width: 10px; height: 10px; border-radius: 50%; }
.install-block pre { margin: 0; padding: 20px 22px; font-family: var(--mono); font-size: 12.5px; line-height: 1.6; overflow-x: auto; color: var(--s-def); }
.install-block .cmd-comment { color: var(--s-com); }
.install-block .cmd-dollar { color: var(--accent-2); }

/* ── Playground teaser ── */
.playground-teaser { border: 1px solid var(--border); border-radius: var(--radius); overflow: hidden; position: relative; }
.playground-teaser-bar { height: 36px; background: var(--panel); border-bottom: 1px solid var(--border); display: flex; align-items: center; padding: 0 14px; gap: 8px; }
.playground-teaser-dot { width: 10px; height: 10px; border-radius: 50%; }
.playground-preview { display: grid; grid-template-columns: 1fr 1fr; min-height: 200px; background: var(--panel-3); }
.playground-preview-pane { padding: 16px; font-family: var(--mono); font-size: 11.5px; line-height: 1.5; color: var(--s-def); border-right: 1px solid var(--border); overflow: hidden; }
.playground-preview-pane:last-child { border-right: none; }
.playground-teaser-overlay { position: absolute; inset: 0; background: linear-gradient(180deg, transparent 30%, rgba(15,15,15,0.95) 80%); display: flex; flex-direction: column; align-items: center; justify-content: flex-end; padding: 28px; gap: 12px; }
.playground-teaser-overlay p { font-size: var(--body); color: var(--text-dim); margin: 0; text-align: center; }

/* ── Section titles ── */
.section-title { font-size: var(--h2); font-weight: 700; letter-spacing: -0.02em; color: var(--text); margin-bottom: 8px; }
.section-sub { font-size: var(--body); color: var(--text-dim); line-height: var(--lh); margin-bottom: 32px; }
.section { padding: clamp(48px, 7vw, 80px) 0; }
.section + .section { border-top: 1px solid var(--border); }

/* ── Solidity coverage table ── */
.coverage-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 24px; }
@media (max-width: 640px) { .coverage-grid { grid-template-columns: 1fr; } }
.coverage-col h4 { font-size: 12px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.07em; color: var(--text-dim); margin-bottom: 12px; }
.coverage-list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 6px; }
.coverage-list li { font-family: var(--mono); font-size: 12px; color: var(--text-dim); display: flex; align-items: center; gap: 8px; }
.coverage-list li::before { content: ''; width: 6px; height: 6px; border-radius: 50%; flex-shrink: 0; }
.coverage-ok li::before { background: var(--green); }
.coverage-no li::before { background: var(--red); opacity: 0.6; }

/* ── Contract cards ── */
.contracts-filter { display: flex; gap: 8px; flex-wrap: wrap; margin-bottom: 28px; }
.filter-btn { font-family: var(--mono); font-size: 11px; padding: 5px 12px; border: 1px solid var(--border-2); border-radius: 20px; background: var(--panel); color: var(--text-dim); cursor: pointer; transition: all 0.12s; }
.filter-btn:hover, .filter-btn.active { border-color: var(--accent); color: var(--accent-2); background: var(--accent-dim); }
.contracts-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 16px; }
.contract-card { background: var(--panel); border: 1px solid var(--border); border-radius: var(--radius); padding: 20px; display: flex; flex-direction: column; gap: 10px; transition: border-color 0.15s, background 0.15s; }
.contract-card:hover { border-color: var(--border-2); background: var(--panel-2); }
.contract-card-head { display: flex; align-items: center; gap: 8px; }
.contract-name { font-weight: 700; font-size: 14px; color: var(--text); font-family: var(--mono); }
.contract-tag { font-family: var(--mono); font-size: 9px; text-transform: uppercase; letter-spacing: 0.05em; padding: 2px 7px; border-radius: 10px; font-weight: 600; }
.tag-basics    { color: #a78bfa; background: rgba(167,139,250,0.1); border: 1px solid rgba(167,139,250,0.2); }
.tag-math      { color: #f59e0b; background: rgba(245,158,11,0.1);  border: 1px solid rgba(245,158,11,0.2); }
.tag-tokens    { color: var(--green); background: rgba(16,185,129,0.1); border: 1px solid rgba(16,185,129,0.2); }
.tag-payable   { color: #34d399; background: rgba(52,211,153,0.1);  border: 1px solid rgba(52,211,153,0.2); }
.tag-access    { color: var(--accent-2); background: var(--accent-dim); border: 1px solid rgba(59,130,246,0.2); }
.tag-cross-contract { color: #f472b6; background: rgba(244,114,182,0.1); border: 1px solid rgba(244,114,182,0.2); }
.tag-oop       { color: #fb923c; background: rgba(251,146,60,0.1);  border: 1px solid rgba(251,146,60,0.2); }
.tag-strings   { color: #a3e635; background: rgba(163,230,53,0.1);  border: 1px solid rgba(163,230,53,0.2); }
.tag-arrays    { color: #38bdf8; background: rgba(56,189,248,0.1);  border: 1px solid rgba(56,189,248,0.2); }
.contract-desc { font-size: var(--small); color: var(--text-dim); line-height: 1.5; }
.contract-sig  { font-family: var(--mono); font-size: 11px; color: var(--text-faint); }
.contract-footer { display: flex; align-items: center; justify-content: space-between; margin-top: 4px; padding-top: 10px; border-top: 1px solid var(--border); }
.contract-status { font-family: var(--mono); font-size: 10px; color: var(--green); display: flex; align-items: center; gap: 5px; }
.contract-links { display: flex; gap: 8px; }
.contract-link { font-family: var(--mono); font-size: 10px; color: var(--text-faint); text-decoration: none; padding: 3px 8px; border: 1px solid var(--border); border-radius: 4px; transition: color 0.1s, border-color 0.1s; }
.contract-link:hover { color: var(--text-dim); border-color: var(--border-2); }

/* ── Docs layout ── */
.docs-shell { display: flex; flex: 1; min-height: 0; }
.docs-sidebar { width: 240px; flex-shrink: 0; border-right: 1px solid var(--border); padding: 24px 0; overflow-y: auto; position: sticky; top: 60px; height: calc(100vh - 60px); }
.docs-sidebar-section { margin-bottom: 20px; }
.docs-sidebar-section-title { font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.08em; color: var(--text-faint); padding: 0 20px; margin-bottom: 6px; }
.docs-sidebar-link { display: block; font-size: var(--small); color: var(--text-dim); padding: 5px 20px; text-decoration: none; transition: color 0.1s, background 0.1s; }
.docs-sidebar-link:hover { color: var(--text); background: rgba(255,255,255,0.03); }
.docs-sidebar-link.active { color: var(--accent-2); background: var(--accent-dim); border-right: 2px solid var(--accent); }
.docs-content { flex: 1; min-width: 0; padding: 40px clamp(24px, 4vw, 56px) 80px; max-width: 760px; }
.docs-content h1 { font-size: var(--h2); font-weight: 700; letter-spacing: -0.02em; color: var(--text); margin: 0 0 8px; }
.docs-content h2 { font-size: 1.1rem; font-weight: 700; color: var(--text); margin: 36px 0 12px; padding-top: 36px; border-top: 1px solid var(--border); }
.docs-content h2:first-of-type { border-top: none; padding-top: 0; }
.docs-content h3 { font-size: 0.95rem; font-weight: 600; color: var(--text); margin: 24px 0 8px; }
.docs-content p { font-size: var(--body); color: var(--text-dim); line-height: var(--lh); margin: 0 0 14px; }
.docs-content a { color: var(--accent-2); text-decoration: none; }
.docs-content a:hover { text-decoration: underline; }
.docs-content pre { background: var(--panel-3); border: 1px solid var(--border); border-radius: var(--radius-sm); padding: 16px 18px; overflow-x: auto; font-family: var(--mono); font-size: 12.5px; line-height: 1.6; color: var(--s-def); margin: 0 0 16px; }
.docs-content code { font-family: var(--mono); font-size: 12px; background: var(--panel); border: 1px solid var(--border); border-radius: 4px; padding: 1px 5px; color: var(--s-id); }
.docs-content pre code { background: none; border: none; padding: 0; font-size: inherit; color: inherit; }
.docs-content table { width: 100%; border-collapse: collapse; margin: 0 0 20px; font-size: var(--small); }
.docs-content th { text-align: left; font-size: 10px; text-transform: uppercase; letter-spacing: 0.07em; color: var(--text-faint); padding: 8px 12px; border-bottom: 1px solid var(--border); }
.docs-content td { padding: 9px 12px; border-bottom: 1px solid var(--border); color: var(--text-dim); vertical-align: top; }
.docs-content td code { font-size: 11px; }
.docs-content ul, .docs-content ol { padding-left: 20px; margin: 0 0 14px; color: var(--text-dim); font-size: var(--body); line-height: var(--lh); }
.docs-content li { margin-bottom: 4px; }
.docs-breadcrumb { font-family: var(--mono); font-size: 11px; color: var(--text-faint); margin-bottom: 16px; display: flex; align-items: center; gap: 6px; }
.docs-breadcrumb a { color: var(--text-faint); text-decoration: none; }
.docs-breadcrumb a:hover { color: var(--text-dim); }
.docs-tag { font-family: var(--mono); font-size: 10px; text-transform: uppercase; letter-spacing: 0.06em; padding: 3px 8px; border-radius: 4px; background: var(--accent-dim); color: var(--accent-2); border: 1px solid rgba(59,130,246,0.2); margin-bottom: 12px; display: inline-block; }

/* ── Why page ── */
.why-section { padding: clamp(40px, 6vw, 72px) 0; }
.why-section + .why-section { border-top: 1px solid var(--border); }
.three-col { display: grid; grid-template-columns: repeat(3, 1fr); gap: 20px; }
@media (max-width: 720px) { .three-col { grid-template-columns: 1fr; } }
.three-col-card { padding: 20px; background: var(--panel); border: 1px solid var(--border); border-radius: var(--radius); }
.three-col-card h4 { font-size: 12px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-faint); margin-bottom: 10px; }
.three-col-card p { font-size: var(--small); color: var(--text-dim); line-height: var(--lh); margin: 0; }
.integrity-callout { background: var(--panel); border: 1px solid var(--green); border-radius: var(--radius); padding: 28px 32px; text-align: center; }
.integrity-callout h3 { font-size: 1.1rem; font-weight: 700; color: var(--green); margin-bottom: 8px; }
.integrity-callout p { font-size: var(--body); color: var(--text-dim); line-height: var(--lh); margin: 0; }
.limits-list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 10px; }
.limits-list li { display: flex; gap: 10px; font-size: var(--body); color: var(--text-dim); line-height: var(--lh); }
.limits-list li::before { content: '·'; color: var(--text-faint); flex-shrink: 0; margin-top: 1px; }
.cta-row { display: flex; gap: 12px; flex-wrap: wrap; }

/* ── Shared utilities ── */
.mt-4 { margin-top: 16px; }
.mt-6 { margin-top: 24px; }
.mt-8 { margin-top: 32px; }
.mb-4 { margin-bottom: 16px; }
.mb-6 { margin-bottom: 24px; }
.text-center { text-align: center; }
.inline-chip { font-family: var(--mono); font-size: 10.5px; padding: 2px 8px; border: 1px solid var(--border); border-radius: 4px; color: var(--text-faint); }
```

- [ ] **Step 5: Commit**

```bash
cd playground
git add package.json package-lock.json next.config.ts mdx-components.tsx app/'(site)'/site.css
git commit -m "feat(site): MDX setup + marketing site design tokens"
```

---

## Task 2: Route group layout — SiteNav + SiteFooter

**Files:**
- Create: `playground/app/(site)/layout.tsx`
- Create: `playground/components/SiteNav.tsx`
- Create: `playground/components/SiteFooter.tsx`

> **Invoke `ui-ux-pro-max` skill before implementing this task.**

- [ ] **Step 1: Create SiteNav component**

`playground/components/SiteNav.tsx`:
```tsx
'use client';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { Ic } from '@/components/Pipeline';

export default function SiteNav() {
  const path = usePathname();

  return (
    <nav className="site-nav">
      <Link href="/home" className="site-brand">
        <span className="site-brand-mark">
          <Ic.bolt width={13} height={13} />
        </span>
        Ink<strong>Port</strong>
      </Link>
      <div className="site-nav-links">
        <Link href="/docs/getting-started/install" className={path.startsWith('/docs') ? 'active' : ''}>Docs</Link>
        <Link href="/contracts" className={path === '/contracts' ? 'active' : ''}>Contracts</Link>
        <Link href="/why-inkport" className={path === '/why-inkport' ? 'active' : ''}>Why InkPort</Link>
      </div>
      <div className="site-nav-spacer" />
      <Link href="/" className="btn btn-primary site-nav-cta">Try Playground →</Link>
    </nav>
  );
}
```

- [ ] **Step 2: Create SiteFooter component**

`playground/components/SiteFooter.tsx`:
```tsx
export default function SiteFooter() {
  return (
    <footer className="site-footer">
      <span>MIT License</span>
      <span>·</span>
      <a href="https://github.com/freedanjeremiah/inkide" target="_blank" rel="noopener noreferrer">
        GitHub
      </a>
      <span>·</span>
      <span>Portaldot: <span style={{ color: 'var(--text-faint)' }}>wss://portaldot.philotheephilix.in</span></span>
    </footer>
  );
}
```

- [ ] **Step 3: Create (site) layout**

`playground/app/(site)/layout.tsx`:
```tsx
import './site.css';
import SiteNav from '@/components/SiteNav';
import SiteFooter from '@/components/SiteFooter';

export default function SiteLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="site-shell">
      <SiteNav />
      <main style={{ flex: 1 }}>{children}</main>
      <SiteFooter />
    </div>
  );
}
```

- [ ] **Step 4: Verify no conflicts with playground**

Run `npm run dev` in `playground/`. Visit `http://localhost:3000` — playground should be unchanged. Visit `http://localhost:3000/home` — should return 404 (not yet created). No errors in console.

- [ ] **Step 5: Commit**

```bash
git add app/'(site)'/layout.tsx components/SiteNav.tsx components/SiteFooter.tsx
git commit -m "feat(site): route group layout with nav and footer"
```

---

## Task 3: Static data — contracts + docs nav

**Files:**
- Create: `playground/lib/contracts-data.ts`
- Create: `playground/lib/docs-nav.ts`

- [ ] **Step 1: Create contracts-data.ts**

`playground/lib/contracts-data.ts`:
```typescript
export type ContractTag =
  | 'basics' | 'math' | 'tokens' | 'payable' | 'access'
  | 'cross-contract' | 'oop' | 'strings' | 'arrays';

export interface ContractEntry {
  name: string;
  tags: ContractTag[];
  description: string;
  constructor: string;
  messages: string[];
  testSteps: number;
  solFile: string;
  testFile: string;
}

const REPO = 'https://github.com/freedanjeremiah/inkide/blob/main';

export const CONTRACTS: ContractEntry[] = [
  { name: 'Counter', tags: ['basics'], description: 'Stateful increment with constructor arg and view getter.', constructor: 'constructor(uint256 initial)', messages: ['inc()', 'incBy(uint256)', 'get()'], testSteps: 5, solFile: `${REPO}/contracts/Counter.sol`, testFile: `${REPO}/tests/Counter.json` },
  { name: 'Flipper', tags: ['basics'], description: 'Boolean toggle — simplest possible stateful contract.', constructor: 'constructor(bool init)', messages: ['flip()', 'get()'], testSteps: 3, solFile: `${REPO}/contracts/Flipper.sol`, testFile: `${REPO}/tests/Flipper.json` },
  { name: 'SimpleStorage', tags: ['basics'], description: 'Single uint store and retrieve.', constructor: 'constructor()', messages: ['set(uint256)', 'get()'], testSteps: 3, solFile: `${REPO}/contracts/SimpleStorage.sol`, testFile: `${REPO}/tests/SimpleStorage.json` },
  { name: 'Pub', tags: ['basics'], description: 'Public variable auto-getter pattern.', constructor: 'constructor(uint256)', messages: ['value()'], testSteps: 2, solFile: `${REPO}/contracts/Pub.sol`, testFile: `${REPO}/tests/Pub.json` },
  { name: 'Inc', tags: ['basics'], description: 'Increment-only counter.', constructor: 'constructor()', messages: ['inc()', 'get()'], testSteps: 3, solFile: `${REPO}/contracts/Inc.sol`, testFile: `${REPO}/tests/Inc.json` },
  { name: 'Sum', tags: ['math'], description: 'Accumulator with running total.', constructor: 'constructor()', messages: ['add(uint256)', 'total()'], testSteps: 4, solFile: `${REPO}/contracts/Sum.sol`, testFile: `${REPO}/tests/Sum.json` },
  { name: 'MinMax', tags: ['math'], description: 'Tracks minimum and maximum of submitted values.', constructor: 'constructor()', messages: ['submit(uint256)', 'min()', 'max()'], testSteps: 5, solFile: `${REPO}/contracts/MinMax.sol`, testFile: `${REPO}/tests/MinMax.json` },
  { name: 'Bits', tags: ['math'], description: 'Bitwise operators: &, |, ^, ~, <<, >>.', constructor: 'constructor()', messages: ['and(uint256,uint256)', 'or(uint256,uint256)', 'xor(uint256,uint256)'], testSteps: 6, solFile: `${REPO}/contracts/Bits.sol`, testFile: `${REPO}/tests/Bits.json` },
  { name: 'Signed', tags: ['math'], description: 'Signed i128 arithmetic with overflow protection.', constructor: 'constructor(int256)', messages: ['add(int256)', 'get()'], testSteps: 4, solFile: `${REPO}/contracts/Signed.sol`, testFile: `${REPO}/tests/Signed.json` },
  { name: 'NarrowMath', tags: ['math'], description: 'uint16 overflow reverts at declared width.', constructor: 'constructor()', messages: ['add(uint16,uint16)'], testSteps: 4, solFile: `${REPO}/contracts/NarrowMath.sol`, testFile: `${REPO}/tests/NarrowMath.json` },
  { name: 'Narrow16', tags: ['math'], description: 'Width-checked arithmetic on uint16 variables.', constructor: 'constructor(uint16)', messages: ['inc()', 'get()'], testSteps: 3, solFile: `${REPO}/contracts/Narrow16.sol`, testFile: `${REPO}/tests/Narrow16.json` },
  { name: 'Unchecked', tags: ['math'], description: 'unchecked{} wraps instead of reverts.', constructor: 'constructor()', messages: ['wrap(uint256,uint256)'], testSteps: 3, solFile: `${REPO}/contracts/Unchecked.sol`, testFile: `${REPO}/tests/Unchecked.json` },
  { name: 'Cast', tags: ['math'], description: 'Narrowing cast: uint8(256) == 0.', constructor: 'constructor()', messages: ['cast8(uint256)'], testSteps: 3, solFile: `${REPO}/contracts/Cast.sol`, testFile: `${REPO}/tests/Cast.json` },
  { name: 'ERC20', tags: ['tokens'], description: 'Fungible token: transfer, approve, allowance, events.', constructor: 'constructor(uint256 initialSupply)', messages: ['transfer(address,uint256)', 'transferFrom(address,address,uint256)', 'approve(address,uint256)', 'balanceOf(address)', 'allowance(address,address)'], testSteps: 8, solFile: `${REPO}/contracts/ERC20.sol`, testFile: `${REPO}/tests/ERC20.json` },
  { name: 'ERC721', tags: ['tokens'], description: 'Non-fungible token: mint, transfer, ownership.', constructor: 'constructor()', messages: ['mint(address,uint256)', 'transfer(address,uint256)', 'ownerOf(uint256)'], testSteps: 6, solFile: `${REPO}/contracts/ERC721.sol`, testFile: `${REPO}/tests/ERC721.json` },
  { name: 'Ownable', tags: ['access'], description: 'onlyOwner modifier and ownership transfer.', constructor: 'constructor()', messages: ['transferOwnership(address)', 'owner()'], testSteps: 4, solFile: `${REPO}/contracts/Ownable.sol`, testFile: `${REPO}/tests/Ownable.json` },
  { name: 'Bank', tags: ['payable'], description: 'msg.value deposit/withdraw with balance tracking.', constructor: 'constructor()', messages: ['deposit()', 'withdraw(uint256)', 'balanceOf(address)'], testSteps: 5, solFile: `${REPO}/contracts/Bank.sol`, testFile: `${REPO}/tests/Bank.json` },
  { name: 'Escrow', tags: ['payable'], description: 'Conditional release of held funds between parties.', constructor: 'constructor(address beneficiary)', messages: ['deposit()', 'release()'], testSteps: 5, solFile: `${REPO}/contracts/Escrow.sol`, testFile: `${REPO}/tests/Escrow.json` },
  { name: 'Auction', tags: ['payable'], description: 'Timed bidding with highest-bidder tracking.', constructor: 'constructor(uint256 duration)', messages: ['bid()', 'end()', 'highestBidder()', 'highestBid()'], testSteps: 7, solFile: `${REPO}/contracts/Auction.sol`, testFile: `${REPO}/tests/Auction.json` },
  { name: 'Voting', tags: ['access'], description: 'Proposal creation, vote recording, result query.', constructor: 'constructor()', messages: ['addProposal(string)', 'vote(uint256)', 'winner()'], testSteps: 6, solFile: `${REPO}/contracts/Voting.sol`, testFile: `${REPO}/tests/Voting.json` },
  { name: 'Greeter', tags: ['strings'], description: 'string storage and retrieval.', constructor: 'constructor(string memory)', messages: ['greet()', 'setGreeting(string)'], testSteps: 3, solFile: `${REPO}/contracts/Greeter.sol`, testFile: `${REPO}/tests/Greeter.json` },
  { name: 'IntList', tags: ['arrays'], description: 'Dynamic uint[] array with push, length, index access.', constructor: 'constructor()', messages: ['push(uint256)', 'get(uint256)', 'length()'], testSteps: 5, solFile: `${REPO}/contracts/IntList.sol`, testFile: `${REPO}/tests/IntList.json` },
  { name: 'Structs', tags: ['oop'], description: 'Struct locals and field access.', constructor: 'constructor()', messages: ['store(uint256,address)', 'retrieve(uint256)'], testSteps: 4, solFile: `${REPO}/contracts/Structs.sol`, testFile: `${REPO}/tests/Structs.json` },
  { name: 'Enum', tags: ['oop'], description: 'Enum state machine with transitions.', constructor: 'constructor()', messages: ['advance()', 'state()'], testSteps: 4, solFile: `${REPO}/contracts/Enum.sol`, testFile: `${REPO}/tests/Enum.json` },
  { name: 'Inherit', tags: ['oop'], description: 'Inheritance flattening (is Base) with overridden methods.', constructor: 'constructor()', messages: ['value()', 'double()'], testSteps: 3, solFile: `${REPO}/contracts/Inherit.sol`, testFile: `${REPO}/tests/Inherit.json` },
  { name: 'Caller', tags: ['cross-contract'], description: 'Calls into Target via IFoo(addr).bar(args) cross-contract call.', constructor: 'constructor(address target)', messages: ['callTarget(uint256)', 'result()'], testSteps: 5, solFile: `${REPO}/contracts/Caller.sol`, testFile: `${REPO}/tests/Caller.json` },
  { name: 'Target', tags: ['cross-contract'], description: 'Deployed as dependency, referenced via @label in test specs.', constructor: 'constructor()', messages: ['set(uint256)', 'get()'], testSteps: 2, solFile: `${REPO}/contracts/Target.sol`, testFile: `${REPO}/tests/Target.json` },
  { name: 'Overload', tags: ['oop'], description: 'Function overloading with distinct keccak4 selectors.', constructor: 'constructor()', messages: ['add(uint256)', 'add(uint256,uint256)'], testSteps: 4, solFile: `${REPO}/contracts/Overload.sol`, testFile: `${REPO}/tests/Overload.json` },
  { name: 'IdStore', tags: ['strings'], description: 'bytes32 storage keyed by address.', constructor: 'constructor()', messages: ['store(bytes32)', 'load(address)'], testSteps: 3, solFile: `${REPO}/contracts/IdStore.sol`, testFile: `${REPO}/tests/IdStore.json` },
  { name: 'Timed', tags: ['access'], description: 'block.timestamp gating — only callable within time window.', constructor: 'constructor(uint256 openAt, uint256 closeAt)', messages: ['action()', 'isOpen()'], testSteps: 4, solFile: `${REPO}/contracts/Timed.sol`, testFile: `${REPO}/tests/Timed.json` },
];

export const ALL_TAGS: ContractTag[] = ['basics', 'math', 'tokens', 'payable', 'access', 'cross-contract', 'oop', 'strings', 'arrays'];
```

- [ ] **Step 2: Create docs-nav.ts**

`playground/lib/docs-nav.ts`:
```typescript
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
```

- [ ] **Step 3: Commit**

```bash
git add lib/contracts-data.ts lib/docs-nav.ts
git commit -m "feat(site): static contracts data and docs nav structure"
```

---

## Task 4: Landing page (`/home`)

**Files:**
- Create: `playground/app/(site)/home/page.tsx`

> **Invoke `ui-ux-pro-max` skill before implementing. Use Opus 4.8.**

- [ ] **Step 1: Create landing page**

`playground/app/(site)/home/page.tsx`:
```tsx
import Link from 'next/link';
import { Ic } from '@/components/Pipeline';

export const metadata = {
  title: 'InkPort — Write Solidity, Deploy to Portaldot',
  description: 'InkPort translates Solidity to raw seal0 Rust, compiles to WebAssembly, and deploys on the live Portaldot chain.',
};

const PIPELINE_STEPS = [
  { label: 'Translate', sub: 'solang-parser\n→ seal0 Rust', cmd: 'inkport translate' },
  { label: 'Compile',   sub: 'cargo +stable\nwasm32', cmd: 'inkport build' },
  { label: 'Deploy',    sub: 'instantiate\n_with_code', cmd: 'inkport deploy' },
  { label: 'Call',      sub: 'Contracts.call\ndry-run / extrinsic', cmd: 'inkport call' },
];

const SUPPORTED = [
  'bool, uintN→u128, intN→i128, address',
  'mapping(K=>V), nested mappings, T[]',
  'events, modifiers, inheritance',
  'msg.sender, msg.value, block.timestamp',
  'payable functions, cross-contract calls',
  'Function overloading, enums, structs',
];

const REJECTED = [
  'inline assembly',
  'delegatecall',
  'tx.origin',
  'ternary ?:',
  'new ContractFactory()',
  'libraries / using for',
];

const INSTALL_LINES = [
  ['comment', '# 1. Build the Rust translator'],
  ['dollar', 'source "$HOME/.cargo/env"'],
  ['dollar', '(cd translator && cargo build --release)'],
  ['blank', ''],
  ['comment', '# 2. Install the Python CLI'],
  ['dollar', 'python3.11 -m venv .venv && source .venv/bin/activate'],
  ['dollar', 'pip install -e inkport'],
  ['blank', ''],
  ['comment', '# 3. Translate → build → deploy'],
  ['dollar', 'inkport translate contracts/ERC20.sol'],
  ['dollar', 'inkport build ERC20'],
  ['dollar', 'inkport deploy ERC20 --arg 1000000'],
  ['dollar', 'inkport call ERC20 balanceOf --arg //Alice'],
];

export default function HomePage() {
  return (
    <div>
      {/* ── Hero ── */}
      <section className="site-hero">
        <div className="site-container">
          <p className="site-hero-eyebrow">Portaldot · pallet-contracts · seal0</p>
          <h1 className="site-h1">
            Write <span className="gradient-text">Solidity.</span><br />
            Deploy to Portaldot.
          </h1>
          <p className="site-hero-sub">
            InkPort translates Solidity contracts to raw seal0 Rust, compiles them to
            WebAssembly, and deploys + tests them on the live Portaldot chain —
            a Hardhat-style workflow whose compile target is pallet-contracts.
          </p>
          <div className="site-hero-actions">
            <Link href="/" className="btn btn-primary">Try the Playground →</Link>
            <a href="https://github.com/freedanjeremiah/inkide" className="btn" target="_blank" rel="noopener noreferrer">View on GitHub</a>
          </div>
          <p className="inline-chip" style={{ marginTop: 20, display: 'inline-block' }}>sol → seal0 rust → wasm → portaldot</p>
        </div>
      </section>

      {/* ── Pipeline strip ── */}
      <section className="section" style={{ paddingTop: 0, paddingBottom: 48 }}>
        <div className="site-container">
          <div className="pipeline-strip" style={{ justifyContent: 'center' }}>
            <div className="pipeline-step" style={{ background: 'var(--panel-2)', borderColor: 'var(--border-2)', minWidth: 80, padding: '12px 16px', borderRadius: 'var(--radius-sm)' }}>
              <span style={{ fontFamily: 'var(--mono)', fontSize: 13, color: 'var(--s-str)' }}>.sol</span>
            </div>
            {PIPELINE_STEPS.map((s, i) => (
              <span key={s.label} style={{ display: 'contents' }}>
                <span className="pipeline-arrow">→</span>
                <div className="pipeline-step">
                  <span className="pipeline-step-label">{s.label}</span>
                  <span className="pipeline-step-sub">{s.sub}</span>
                  <span className="pipeline-step-cmd">{s.cmd}</span>
                </div>
              </span>
            ))}
          </div>
        </div>
      </section>

      {/* ── Stats ── */}
      <section className="section" style={{ paddingTop: 0 }}>
        <div className="site-container">
          <div className="stats-bar">
            {[
              { num: '30', label: 'contracts validated' },
              { num: '89', label: 'translator tests' },
              { num: '0', label: 'silent miscompiles' },
              { num: '✓', label: 'live on Portaldot' },
            ].map(s => (
              <div key={s.label} className="stat-item">
                <div className="stat-num">{s.num}</div>
                <div className="stat-label">{s.label}</div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* ── Two-column value prop ── */}
      <section className="section">
        <div className="site-container">
          <div className="value-cols">
            <div className="value-col">
              <div className="value-col-title"><span className="vc-dot" style={{ background: 'var(--accent)' }} />For Solidity developers</div>
              <p>You already know Solidity. InkPort lets you deploy it directly to Portaldot without learning ink!, Rust nightly builds, or a new framework from scratch. The CLI mirrors Hardhat: <code>translate → build → deploy → call</code>.</p>
            </div>
            <div className="value-col">
              <div className="value-col-title"><span className="vc-dot" style={{ background: 'var(--green)' }} />For Portaldot builders</div>
              <p>Portaldot's pallet-contracts speaks seal0 — a raw host ABI from ~Substrate 2021. InkPort is the only toolchain that targets it from a high-level language today, compiling on stable Rust with no ink! dependency.</p>
            </div>
          </div>
        </div>
      </section>

      {/* ── Quick install ── */}
      <section className="section">
        <div className="site-container">
          <h2 className="section-title">Quick install</h2>
          <p className="section-sub">Prereqs: rustup (stable + wasm32-unknown-unknown target), Python 3.11.</p>
          <div className="install-block">
            <div className="install-block-header">
              <span className="dot" style={{ background: '#ef4444' }} />
              <span className="dot" style={{ background: '#f59e0b' }} />
              <span className="dot" style={{ background: '#10b981' }} />
              <span style={{ marginLeft: 8 }}>bash</span>
            </div>
            <pre>
              {INSTALL_LINES.map((line, i) => {
                if (line[0] === 'blank') return <span key={i}>{'\n'}</span>;
                if (line[0] === 'comment') return <span key={i} className="cmd-comment">{line[1]}{'\n'}</span>;
                return <span key={i}><span className="cmd-dollar">$ </span>{line[1]}{'\n'}</span>;
              })}
            </pre>
          </div>
          <p className="mt-4" style={{ fontSize: 'var(--small)', color: 'var(--text-faint)' }}>
            <Link href="/docs/getting-started/install" style={{ color: 'var(--accent-2)', textDecoration: 'none' }}>Full install guide →</Link>
          </p>
        </div>
      </section>

      {/* ── Playground teaser ── */}
      <section className="section">
        <div className="site-container">
          <h2 className="section-title">Try it in the browser</h2>
          <p className="section-sub">No install needed. Edit Solidity on the left, watch seal0 Rust generate live on the right.</p>
          <div className="playground-teaser">
            <div className="playground-teaser-bar">
              {['#ef4444','#f59e0b','#10b981'].map(c => <span key={c} className="playground-teaser-dot" style={{ background: c }} />)}
              <span style={{ fontFamily: 'var(--mono)', fontSize: 11, color: 'var(--text-faint)', marginLeft: 8 }}>InkPort Playground — sol → seal0 → wasm</span>
            </div>
            <div className="playground-preview">
              <div className="playground-preview-pane">
                <span style={{ color: 'var(--s-com)' }}>{'// contracts/Counter.sol\n'}</span>
                <span style={{ color: 'var(--s-kw)' }}>{'contract '}</span>
                <span style={{ color: 'var(--s-type)' }}>{'Counter '}</span>
                <span>{'{\n'}</span>
                <span style={{ color: 'var(--s-id)' }}>{'  uint256 '}</span>
                <span>{'private count;\n\n'}</span>
                <span style={{ color: 'var(--s-fn)' }}>{'  function '}</span>
                <span>{'inc() '}</span>
                <span style={{ color: 'var(--s-kw)' }}>{'public '}</span>
                <span>{'{\n'}</span>
                <span>{'    count += 1;\n  }\n'}</span>
                <span style={{ color: 'var(--s-fn)' }}>{'  function '}</span>
                <span>{'get() '}</span>
                <span style={{ color: 'var(--s-kw)' }}>{'public view\n  returns (uint256) '}</span>
                <span>{'{ return count; }\n}'}</span>
              </div>
              <div className="playground-preview-pane">
                <span style={{ color: 'var(--s-com)' }}>{'// build/Counter/src/lib.rs\n'}</span>
                <span style={{ color: 'var(--s-attr)' }}>{'#![no_std]\n#![no_main]\n\n'}</span>
                <span style={{ color: 'var(--s-kw)' }}>{'mod '}</span>
                <span style={{ color: 'var(--s-type)' }}>{'seal0 '}</span>
                <span>{'{\n'}</span>
                <span style={{ color: 'var(--s-attr)' }}>{'  #[link(wasm_import_module="seal0")]\n'}</span>
                <span style={{ color: 'var(--s-kw)' }}>{'  extern "C" '}</span>
                <span>{'{\n'}</span>
                <span style={{ color: 'var(--text-faint)' }}>{'    pub fn seal_input(...);\n    pub fn seal_return(...);\n    ...\n  }\n}'}</span>
              </div>
            </div>
            <div className="playground-teaser-overlay">
              <p>Compile → deploy → call, all simulated in-session.</p>
              <Link href="/" className="btn btn-primary">Open Playground →</Link>
            </div>
          </div>
        </div>
      </section>

      {/* ── Supported Solidity ── */}
      <section className="section">
        <div className="site-container">
          <h2 className="section-title">Supported Solidity</h2>
          <p className="section-sub">
            Every construct either compiles to semantically-correct seal0 Rust or <code>inkport translate</code> exits non-zero — never a silent miscompile.
          </p>
          <div className="coverage-grid">
            <div className="coverage-col">
              <h4>Supported</h4>
              <ul className="coverage-list coverage-ok">
                {SUPPORTED.map(s => <li key={s}>{s}</li>)}
              </ul>
            </div>
            <div className="coverage-col">
              <h4>Rejected (fail-loud)</h4>
              <ul className="coverage-list coverage-no">
                {REJECTED.map(s => <li key={s}>{s}</li>)}
              </ul>
            </div>
          </div>
          <p className="mt-6" style={{ fontSize: 'var(--small)', color: 'var(--text-faint)' }}>
            <Link href="/docs/solidity/supported" style={{ color: 'var(--accent-2)', textDecoration: 'none' }}>Full coverage table →</Link>
          </p>
        </div>
      </section>
    </div>
  );
}
```

- [ ] **Step 2: Verify page renders**

Run `npm run dev`. Visit `http://localhost:3000/home`. Confirm all 7 sections render, nav shows active state for none (home link not in nav), footer visible. No TypeScript errors.

- [ ] **Step 3: Commit**

```bash
git add app/'(site)'/home/page.tsx
git commit -m "feat(site): landing page with hero, pipeline, stats, install, playground teaser"
```

---

## Task 5: Why InkPort page (`/why-inkport`)

**Files:**
- Create: `playground/app/(site)/why-inkport/page.tsx`

> **Use Opus 4.8. Invoke `ui-ux-pro-max` skill.**

- [ ] **Step 1: Create why-inkport page**

`playground/app/(site)/why-inkport/page.tsx`:
```tsx
import Link from 'next/link';

export const metadata = {
  title: 'Why InkPort — seal0 vs ink!, design decisions',
  description: 'Why InkPort targets raw seal0 Rust instead of ink!, the integrity guarantee, and why Solidity.',
};

export default function WhyInkPortPage() {
  return (
    <div className="site-container" style={{ paddingTop: 48, paddingBottom: 80 }}>

      {/* Header */}
      <div style={{ maxWidth: 680, marginBottom: 56 }}>
        <p className="site-hero-eyebrow" style={{ justifyContent: 'flex-start', marginBottom: 16 }}>Design decisions</p>
        <h1 className="site-h1" style={{ textAlign: 'left', fontSize: 'var(--h2)', marginBottom: 16 }}>Why InkPort</h1>
        <p style={{ fontSize: '1rem', color: 'var(--text-dim)', lineHeight: 1.65 }}>
          The reasoning behind a Solidity toolchain that targets raw seal0 — not ink!.
        </p>
      </div>

      {/* Section 1: The problem */}
      <div className="why-section">
        <h2 className="section-title">The problem</h2>
        <div style={{ maxWidth: 680 }}>
          <p style={{ fontSize: 'var(--body)', color: 'var(--text-dim)', lineHeight: 1.65 }}>
            Portaldot runs a rent-era <code>pallet-contracts</code> node — a version of Substrate's smart contract
            pallet from ~2021 that speaks the seal0 host ABI. ink! 3, 4, and 5 each require an era-matched Rust
            nightly and dependency set that doesn't build against this node. ink! 5.x wasm uses host functions this
            node rejects outright.
          </p>
          <p style={{ fontSize: 'var(--body)', color: 'var(--text-dim)', lineHeight: 1.65 }}>
            If you want to deploy a smart contract to Portaldot today, you need to write raw seal0 Rust by hand —
            a <code>no_std</code> crate that imports the node's host functions directly.
            That's what InkPort generates for you.
          </p>
        </div>
      </div>

      {/* Section 2: What seal0 means */}
      <div className="why-section">
        <h2 className="section-title">What "seal0 Rust" means</h2>
        <div className="three-col">
          <div className="three-col-card">
            <h4>The node expects</h4>
            <p>MVP WebAssembly (no memory.fill, no memory.copy), call / deploy exports + imported memory, seal0 host functions: seal_input, seal_return, seal_get_storage, seal_set_storage, seal_deposit_event.</p>
          </div>
          <div className="three-col-card">
            <h4>InkPort emits</h4>
            <p><code>#![no_std]</code> Rust compiled on stable cargo. No nightly, no ink! dependency. Calls seal0 host functions directly. Buffers sized to SCALE payload. Stripped to MVP wasm with imported memory + max declared.</p>
          </div>
          <div className="three-col-card">
            <h4>You write</h4>
            <p>Plain Solidity (.sol file). A single concrete contract. The 30 validated contracts range from a simple counter to ERC20, ERC721, payable escrow, cross-contract calls, and overloading.</p>
          </div>
        </div>
      </div>

      {/* Section 3: Integrity guarantee */}
      <div className="why-section">
        <h2 className="section-title">The integrity guarantee</h2>
        <div className="integrity-callout" style={{ marginBottom: 28 }}>
          <h3>Every construct either compiles to semantically-correct seal0 Rust,<br />or <code style={{ color: 'inherit', background: 'transparent', border: 'none', padding: 0, fontSize: 'inherit', fontFamily: 'var(--mono)' }}>inkport translate</code> exits non-zero.</h3>
          <p>There are no silent miscompiles.</p>
        </div>
        <div style={{ maxWidth: 680, fontSize: 'var(--body)', color: 'var(--text-dim)', lineHeight: 1.65 }}>
          <p>
            Unsupported constructs (<code>delegatecall</code>, <code>assembly</code>, <code>tx.origin</code>,
            ternary <code>?:</code>) produce a clear error and nothing is emitted. The integrity guarantee was hardened
            across an adversarial review loop covering: integer widths (<code>uint8 255+1</code> reverts,{' '}
            <code>unchecked{}</code> wraps to 0, <code>uint8(256)==0</code> narrowing cast), function overloading
            (distinct keccak4 selectors per signature), inheritance flattening, cross-contract <code>seal_call</code>,
            and events (keccak topic + SCALE data).
          </p>
          <p>
            All 30 validated contracts passed a reviewer-verified adversarial loop of 6 rounds — 89 translator
            unit tests green, 0 silent miscompiles detected.
          </p>
        </div>
      </div>

      {/* Section 4: Why Solidity */}
      <div className="why-section">
        <h2 className="section-title">Why Solidity, not a new language</h2>
        <div style={{ maxWidth: 680, fontSize: 'var(--body)', color: 'var(--text-dim)', lineHeight: 1.65 }}>
          <p>
            Solidity is the language tens of thousands of smart contract developers already know. A new DSL would
            require learning new syntax, new idioms, and new tooling. InkPort gives Portaldot developers
            Hardhat's workflow: write <code>contracts/</code>, run <code>inkport deploy</code>, get an address.
          </p>
          <p>
            The translation is source-to-source — Solidity AST → seal0 Rust — not EVM-on-WASM emulation. The output
            is idiomatic, readable Rust that you can inspect, audit, and extend.
          </p>
        </div>
      </div>

      {/* Section 5: What InkPort is NOT */}
      <div className="why-section">
        <h2 className="section-title">What InkPort is not</h2>
        <ul className="limits-list" style={{ maxWidth: 680 }}>
          {[
            ['A production security auditor', 'Translated output should be reviewed before deployment with real value.'],
            ['A full Solidity compiler', 'Inline assembly, delegatecall, libraries, and new ContractFactory() are intentionally rejected.'],
            ['An EVM emulator', 'Gas accounting, storage layout, and ABI encoding differ from Ethereum. The integrity guarantee covers semantic correctness on pallet-contracts, not EVM parity.'],
            ['An ink! replacement', "If your node supports ink! 5.x, use ink!. InkPort exists specifically for rent-era pallet-contracts nodes that reject ink! wasm."],
          ].map(([title, body]) => (
            <li key={title} style={{ flexDirection: 'column', gap: 2 }}>
              <strong style={{ color: 'var(--text)', fontSize: 'var(--body)' }}>{title}</strong>
              <span>{body}</span>
            </li>
          ))}
        </ul>
      </div>

      {/* CTA */}
      <div className="why-section" style={{ paddingBottom: 0, borderTop: 'none' }}>
        <h2 className="section-title">Ready to deploy?</h2>
        <div className="cta-row">
          <Link href="/docs/getting-started/install" className="btn btn-primary">Read the getting started guide →</Link>
          <Link href="/" className="btn">Try the playground</Link>
        </div>
      </div>

    </div>
  );
}
```

- [ ] **Step 2: Verify**

Visit `http://localhost:3000/why-inkport`. All 6 sections visible, nav shows "Why InkPort" as active, no errors.

- [ ] **Step 3: Commit**

```bash
git add app/'(site)'/why-inkport/page.tsx
git commit -m "feat(site): why-inkport page — seal0 story, integrity guarantee"
```

---

## Task 6: Contracts showcase page (`/contracts`)

**Files:**
- Create: `playground/components/ContractCard.tsx`
- Create: `playground/app/(site)/contracts/page.tsx`

> **Use Opus 4.8.**

- [ ] **Step 1: Create ContractCard component**

`playground/components/ContractCard.tsx`:
```tsx
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
```

- [ ] **Step 2: Create contracts page**

`playground/app/(site)/contracts/page.tsx`:
```tsx
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
```

- [ ] **Step 3: Verify**

Visit `http://localhost:3000/contracts`. All 30 cards render. Filter buttons work. "Contracts" nav link shows active. No TypeScript errors.

- [ ] **Step 4: Commit**

```bash
git add components/ContractCard.tsx app/'(site)'/contracts/page.tsx
git commit -m "feat(site): contracts showcase page with filter by tag"
```

---

## Task 7: Docs layout + sidebar + redirect

**Files:**
- Create: `playground/app/(site)/docs/layout.tsx`
- Create: `playground/app/(site)/docs/page.tsx`
- Create: `playground/components/DocsSidebar.tsx`

- [ ] **Step 1: Create DocsSidebar component**

`playground/components/DocsSidebar.tsx`:
```tsx
'use client';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { DOCS_NAV } from '@/lib/docs-nav';

export default function DocsSidebar() {
  const path = usePathname();
  return (
    <aside className="docs-sidebar">
      {DOCS_NAV.map(section => (
        <div key={section.title} className="docs-sidebar-section">
          <div className="docs-sidebar-section-title">{section.title}</div>
          {section.items.map(item => (
            <Link
              key={item.href}
              href={item.href}
              className={`docs-sidebar-link${path === item.href ? ' active' : ''}`}
            >
              {item.label}
            </Link>
          ))}
        </div>
      ))}
    </aside>
  );
}
```

- [ ] **Step 2: Create docs layout**

`playground/app/(site)/docs/layout.tsx`:
```tsx
import DocsSidebar from '@/components/DocsSidebar';

export default function DocsLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="docs-shell">
      <DocsSidebar />
      <div className="docs-content">{children}</div>
    </div>
  );
}
```

- [ ] **Step 3: Create docs root redirect**

`playground/app/(site)/docs/page.tsx`:
```tsx
import { redirect } from 'next/navigation';
export default function DocsPage() {
  redirect('/docs/getting-started/install');
}
```

- [ ] **Step 4: Verify**

Visit `http://localhost:3000/docs` → should redirect to `/docs/getting-started/install` (404 for now, redirect works). Sidebar renders in layout. No errors.

- [ ] **Step 5: Commit**

```bash
git add components/DocsSidebar.tsx app/'(site)'/docs/layout.tsx app/'(site)'/docs/page.tsx
git commit -m "feat(site): docs layout with sidebar and root redirect"
```

---

## Task 8: Getting Started docs

**Files:**
- Create: `playground/app/(site)/docs/getting-started/install/page.mdx`
- Create: `playground/app/(site)/docs/getting-started/first-contract/page.mdx`
- Create: `playground/app/(site)/docs/getting-started/project-layout/page.mdx`

- [ ] **Step 1: Install page**

`playground/app/(site)/docs/getting-started/install/page.mdx`:
````mdx
export const metadata = { title: 'Install — InkPort Docs' };

<div className="docs-tag">Getting Started</div>

# Install

**Prerequisites:** rustup (stable channel), Python 3.11.

## 1. Install the wasm32 target

```bash
rustup target add wasm32-unknown-unknown
```

## 2. Build the Rust translator

```bash
source "$HOME/.cargo/env"
cd translator && cargo build --release
```

The binary is placed at `translator/target/release/inkport-translate`. The CLI discovers it automatically.

## 3. Install the Python CLI

```bash
python3.11 -m venv .venv
source .venv/bin/activate
pip install -e inkport
```

This installs the `inkport` command and its chain client (`substrate-interface` included).

## 4. Verify

```bash
inkport --help
# → translate  build  deploy  call  test  all
```

## Environment

Both `source` commands must be active in your shell before running any `inkport` command:

```bash
source "$HOME/.cargo/env"                              # Rust
source /path/to/inkport/.venv/bin/activate             # Python
```

Add them to your shell profile (`~/.bashrc` / `~/.zshrc`) or a `direnv` setup.
````

- [ ] **Step 2: First contract page**

`playground/app/(site)/docs/getting-started/first-contract/page.mdx`:
````mdx
export const metadata = { title: 'First Contract — InkPort Docs' };

<div className="docs-tag">Getting Started</div>

# Your first contract

A complete Counter contract from Solidity to on-chain in 4 commands.

## 1. Write the contract

`contracts/Counter.sol`:
```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract Counter {
    uint256 private count;

    constructor(uint256 initial) {
        count = initial;
    }

    function inc() public {
        count += 1;
    }

    function get() public view returns (uint256) {
        return count;
    }
}
```

## 2. Translate to seal0 Rust

```bash
inkport translate contracts/Counter.sol
```

Output:
```
build/Counter/src/lib.rs        ← generated seal0 Rust
build/Counter/metadata.json     ← selectors, args, return types
```

## 3. Build to wasm

```bash
inkport build Counter
```

Output:
```
build/Counter/Counter.wasm      ← stripped MVP wasm
```

## 4. Deploy

```bash
inkport deploy Counter --arg 0
# → deployed Counter -> 5D…
```

## 5. Call and read

```bash
inkport call Counter inc
# → (mutating extrinsic, no return)

inkport call Counter get
# → 1
```

## What just happened

`translate` parsed the Solidity AST, mapped types (`uint256` → `u128`), and emitted a `#![no_std]` Rust crate that imports the node's seal0 host functions. `build` ran `cargo +stable build --target wasm32-unknown-unknown` and stripped the result to MVP wasm. `deploy` called `instantiate_with_code` on the Portaldot node. `call` submitted a `Contracts.call` extrinsic.
````

- [ ] **Step 3: Project layout page**

`playground/app/(site)/docs/getting-started/project-layout/page.mdx`:
````mdx
export const metadata = { title: 'Project Layout — InkPort Docs' };

<div className="docs-tag">Getting Started</div>

# Project layout

```
inkport/
  contracts/          Solidity source files (.sol)
  tests/              On-chain test specs (<Name>.json)
  build/              Generated (gitignored)
    <Name>/
      src/lib.rs      Generated seal0 Rust
      metadata.json   Selectors, args, return types
      <Name>.wasm     Stripped MVP wasm
  translator/         Rust crate: solang-parser → seal0 codegen
  inkport/            Python CLI package (inkport command)
  inkport_chain/      Chain client: deploy, call, test harness
  deployments/        Deployed addresses (portaldot.json)
  inkport.config.py   Network + signer configuration
  docs/               Documentation
```

| Path | Role |
|---|---|
| `translator/src/parse.rs` | solang-parser AST ingestion, inheritance flattening |
| `translator/src/lower.rs` | AST → IR |
| `translator/src/codegen_seal.rs` | IR → seal0 Rust + metadata.json |
| `inkport/inkport/cli.py` | typer CLI: translate/build/deploy/call/test/all |
| `inkport/inkport/pipeline.py` | Path resolution, binary discovery, arg coercion |
| `inkport_chain/portaldot.py` | Deploy/call/read over substrate-interface with reconnect |
| `inkport_chain/strip_wasm.py` | MVP-wasm stripper (removes bulk-memory, unknown exports) |
| `inkport_chain/test_contract.py` | Metadata-driven encode/decode + assertion harness |

## inkport.config.py

```python
config = {
    "networks": {
        "portaldot": {
            "url": "wss://portaldot.philotheephilix.in",
            "decimals": 14,
            "ss58": 42,
        },
    },
    "default_signer": "//Alice",
}
```

The CLI reads this file from the repo root. `//Alice`, `//Bob`, `//Charlie` are accepted as signers — they resolve to the 32-byte AccountId of the pre-funded dev accounts.
````

- [ ] **Step 4: Verify**

Visit `http://localhost:3000/docs/getting-started/install`. Content renders, sidebar shows active state for "Install". All 3 pages accessible.

- [ ] **Step 5: Commit**

```bash
git add app/'(site)'/docs/getting-started/
git commit -m "feat(docs): getting started — install, first contract, project layout"
```

---

## Task 9: CLI Reference docs

**Files (6 pages):**
- `playground/app/(site)/docs/cli/translate/page.mdx`
- `playground/app/(site)/docs/cli/build/page.mdx`
- `playground/app/(site)/docs/cli/deploy/page.mdx`
- `playground/app/(site)/docs/cli/call/page.mdx`
- `playground/app/(site)/docs/cli/test/page.mdx`
- `playground/app/(site)/docs/cli/all/page.mdx`

- [ ] **Step 1: translate page**

`playground/app/(site)/docs/cli/translate/page.mdx`:
````mdx
export const metadata = { title: 'inkport translate — CLI Reference' };

<div className="docs-tag">CLI Reference</div>

# inkport translate

Translate a Solidity source file to seal0 Rust + `metadata.json`.

```bash
inkport translate <file.sol> [--out <dir>]
```

| Flag | Default | Description |
|---|---|---|
| `<file.sol>` | required | Path to the Solidity source file |
| `--out <dir>` | `build/<Name>/` | Output directory |

**Output files:**

| File | Description |
|---|---|
| `build/<Name>/src/lib.rs` | Generated seal0 Rust crate (`#![no_std]`) |
| `build/<Name>/metadata.json` | Contract name, constructor args, messages (selector, args, ret, mutates, payable), events |

**Exit codes:**

| Code | Meaning |
|---|---|
| `0` | Translation succeeded |
| `1` | Unsupported construct detected — printed to stderr, nothing emitted |

**Example:**

```bash
inkport translate contracts/ERC20.sol
# → build/ERC20/src/lib.rs
# → build/ERC20/metadata.json
```

**Fail-loud example:**

```bash
inkport translate contracts/Bad.sol
# error: `delegatecall` is rejected (no host function on rent-era pallet-contracts)
# exit 1 — nothing written
```
````

- [ ] **Step 2: build page**

`playground/app/(site)/docs/cli/build/page.mdx`:
````mdx
export const metadata = { title: 'inkport build — CLI Reference' };

<div className="docs-tag">CLI Reference</div>

# inkport build

Compile a translated contract to stripped MVP wasm.

```bash
inkport build <Name>
```

Runs `cargo +stable build --release --target wasm32-unknown-unknown` on `build/<Name>/`, then strips the wasm to the MVP subset the Portaldot node accepts.

**What "stripped" means:** The node rejects `memory.fill` / `memory.copy` (bulk-memory) and unknown exports (`__data_end`, `__heap_base`). The stripper keeps only `call` / `deploy` exports + imported memory with max declared.

**Output:**

```
build/<Name>/<Name>.wasm    ← stripped, Portaldot-ready
```

**Example:**

```bash
inkport build ERC20
# ✓ ERC20.wasm — 4 321 bytes stripped
```
````

- [ ] **Step 3: deploy page**

`playground/app/(site)/docs/cli/deploy/page.mdx`:
````mdx
export const metadata = { title: 'inkport deploy — CLI Reference' };

<div className="docs-tag">CLI Reference</div>

# inkport deploy

Deploy a compiled contract to the Portaldot chain.

```bash
inkport deploy <Name> [--arg <value>]... [--signer <uri>] [--value <POT>]
```

| Flag | Default | Description |
|---|---|---|
| `<Name>` | required | Contract name (must have `build/<Name>/` present) |
| `--arg <value>` | — | Constructor argument (repeat for each arg) |
| `--signer <uri>` | `//Alice` | SURI or SS58 address of the signer |
| `--value <POT>` | `0` | Endowment in POT (14 decimals) |

Calls `instantiate_with_code` on `wss://portaldot.philotheephilix.in`. The contract address is printed on success and saved to `deployments/portaldot.json`.

**Example:**

```bash
inkport deploy ERC20 --arg 1000000
# deployed ERC20 -> 5D1gqjjMi4...
```

**Dev signers:** `//Alice`, `//Bob`, `//Charlie` resolve to the pre-funded Portaldot dev accounts.

**Sequential constraint:** A single signer (`//Alice`) means concurrent deploys cause nonce conflicts. Never run two `inkport deploy` or `inkport all` processes simultaneously.
````

- [ ] **Step 4: call page**

`playground/app/(site)/docs/cli/call/page.mdx`:
````mdx
export const metadata = { title: 'inkport call — CLI Reference' };

<div className="docs-tag">CLI Reference</div>

# inkport call

Call a message on a deployed contract.

```bash
inkport call <Name> <message> [--arg <value>]... [--signer <uri>] [--value <POT>]
```

| Flag | Default | Description |
|---|---|---|
| `<Name>` | required | Contract name |
| `<message>` | required | Message name as defined in `metadata.json` |
| `--arg <value>` | — | Message argument (repeat for each) |
| `--signer <uri>` | `//Alice` | Signer SURI |
| `--value <POT>` | `0` | Value in POT for payable messages |

**View (non-mutating):** executes as a dry-run read — free, instant, prints decoded return value.

**Mutating:** submits a real `Contracts.call` extrinsic, waits for inclusion, prints events.

**Examples:**

```bash
inkport call ERC20 balanceOf --arg //Alice
# → 1000000

inkport call ERC20 transfer --arg //Bob --arg 250
# → Transfer(from=//Alice, to=//Bob, value=250)

inkport call Bank deposit --value 5
# → (payable call, 5 POT transferred)
```
````

- [ ] **Step 5: test page**

`playground/app/(site)/docs/cli/test/page.mdx`:
````mdx
export const metadata = { title: 'inkport test — CLI Reference' };

<div className="docs-tag">CLI Reference</div>

# inkport test

Run a contract's on-chain test spec.

```bash
inkport test <Name>
```

Deploys a fresh instance of `<Name>` and executes each step in `tests/<Name>.json` against the live Portaldot node. Prints `PASS` / `FAIL` per step.

**Example:**

```bash
inkport test ERC20
# deploy ERC20 -> 5D…  PASS
# read balanceOf(//Alice) → 1000000  PASS
# call transfer(//Bob, 1000)  PASS
# event Transfer(from=//Alice, to=//Bob, value=1000)  PASS
# read balanceOf(//Bob) → 1000  PASS
```

See [Test spec format](/docs/reference/test-spec) for the full step schema.
````

- [ ] **Step 6: all page**

`playground/app/(site)/docs/cli/all/page.mdx`:
````mdx
export const metadata = { title: 'inkport all — CLI Reference' };

<div className="docs-tag">CLI Reference</div>

# inkport all

Translate, build, deploy, and test every contract in `contracts/` that has a matching `tests/*.json`.

```bash
inkport all
```

This is the full regression suite. It runs sequentially (one signer, one deploy at a time) and prints a summary:

```
========== inkport all: SUMMARY ==========
  Counter      PASS  (5 steps)
  Flipper      PASS  (3 steps)
  ERC20        PASS  (8 steps)
  ...
  ALL PASS  (30/30)
```

**Warning:** `inkport all` deploys fresh instances of every contract. Do not run it concurrently with another `inkport all` or `inkport deploy` — nonce conflicts will cause failures.

**Runtime:** ~10–20 minutes for 30 contracts on the live Portaldot node over WSS.
````

- [ ] **Step 7: Commit**

```bash
git add app/'(site)'/docs/cli/
git commit -m "feat(docs): CLI reference — translate, build, deploy, call, test, all"
```

---

## Task 10: Solidity coverage docs

**Files:**
- `playground/app/(site)/docs/solidity/supported/page.mdx`
- `playground/app/(site)/docs/solidity/rejected/page.mdx`

- [ ] **Step 1: Supported surface page**

`playground/app/(site)/docs/solidity/supported/page.mdx`:
````mdx
export const metadata = { title: 'Supported Solidity — InkPort Docs' };

<div className="docs-tag">Solidity Coverage</div>

# Supported Solidity surface

| Area | Supported |
|---|---|
| Scalar types | `bool`, `uintN`→`u128` (width-checked), `intN`→`i128` (width-checked), `address`→`AccountId`, `bytes`/`string` (compact-length, trailing param), `bytes32` |
| Collections | `mapping(K=>V)` (address & scalar keys), nested mappings, dynamic arrays `T[]` (`.push`/`.length`/index), `mapping(K=>Struct)` field access |
| Functions | constructor, view / mutating / `payable`, public-var auto-getters, multiple returns, function overloading |
| Statements | assignment, compound assign (`+= -= *= /= %= \|= &= ^= <<= >>=`), `++`/`--`, `if/else`, `for`/`while`/`do-while`, `return`, `emit`, `require`/`assert`/`revert`, local vars, `unchecked {}` |
| Expressions | arithmetic (checked → revert at declared width), comparisons, logical (`&& \|\| !`), bitwise/shift, narrowing casts `uintN(x)` (truncate), literals |
| Integer semantics | true bit-width: `uint8 255+1` reverts; `unchecked{}` wraps to 0; `uint8(256)==0` |
| Context vars | `msg.sender`, `msg.value`, `block.timestamp`, `block.number`, `address(this).balance` |
| Events | `emit E(...)` → `seal_deposit_event` (keccak topic + SCALE data), decoded in the harness |
| OOP | inheritance / interface flattening (`is`), modifiers (inlined as guards), enums, struct locals |
| Cross-contract | `IFoo(addr).bar(args)` via `seal_call` with keccak4 selector |
| ABI | keccak256 4-byte selectors + keccak event-signature topics — same as Ethereum |
````

- [ ] **Step 2: Rejected constructs page**

`playground/app/(site)/docs/solidity/rejected/page.mdx`:
````mdx
export const metadata = { title: 'Rejected Constructs — InkPort Docs' };

<div className="docs-tag">Solidity Coverage</div>

# Rejected constructs

When `inkport translate` encounters any of the following, it exits non-zero with a descriptive error. Nothing is emitted.

| Construct | Error message |
|---|---|
| `assembly { ... }` | `unsupported: inline assembly` |
| `delegatecall` | `unsupported: delegatecall (no host function on rent-era pallet-contracts)` |
| `tx.origin` | `unsupported: tx.origin is not available under seal0` |
| `abi.encodePacked(...)` | `unsupported: abi.encodePacked — use typed args` |
| `library Foo { ... }` | `unsupported: library definitions — flatten into the contract` |
| ternary `a ? b : c` | `unsupported: ternary ?: — use if/else` |
| `new Foo(...)` | `unsupported: new factory deployment` |
| struct in array | `unsupported: struct-in-array storage` |
| nested structs | `unsupported: nested struct types` |
| `string`/`bytes` as non-trailing param | `unsupported: string/bytes param must be last` |
| `tx.gasprice`, `block.coinbase` | `unsupported: EVM-only context variable` |
| struct return across ABI | `unsupported: struct ABI return type` |

## Why fail-loud?

A wrong-but-compiling contract is worse than a rejected one. Silently dropping a modifier or miscompiling an overflow check would produce a contract that deploys successfully but behaves incorrectly. InkPort's integrity guarantee is: if `translate` exits 0, the output is semantically correct. If it can't guarantee that, it exits 1.
````

- [ ] **Step 3: Commit**

```bash
git add app/'(site)'/docs/solidity/
git commit -m "feat(docs): Solidity coverage — supported surface and rejected constructs"
```

---

## Task 11: Feature guides

**Files:**
- `playground/app/(site)/docs/guides/erc20/page.mdx`
- `playground/app/(site)/docs/guides/payable/page.mdx`
- `playground/app/(site)/docs/guides/cross-contract/page.mdx`
- `playground/app/(site)/docs/guides/integers/page.mdx`

- [ ] **Step 1: ERC20 walkthrough**

`playground/app/(site)/docs/guides/erc20/page.mdx`:
````mdx
export const metadata = { title: 'ERC20 Walkthrough — InkPort Guides' };

<div className="docs-tag">Guide</div>

# ERC20 walkthrough

Deploying a standard fungible token end-to-end.

## The contract

`contracts/ERC20.sol` (abridged):
```solidity
contract ERC20 {
    mapping(address => uint256) private balances;
    mapping(address => mapping(address => uint256)) private allowances;
    uint256 public totalSupply;

    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);

    constructor(uint256 initialSupply) {
        totalSupply = initialSupply;
        balances[msg.sender] = initialSupply;
    }

    function transfer(address to, uint256 value) public returns (bool) {
        require(balances[msg.sender] >= value, "insufficient");
        balances[msg.sender] -= value;
        balances[to] += value;
        emit Transfer(msg.sender, to, value);
        return true;
    }
    // ...
}
```

## What the translator produces

The nested `mapping(address => mapping(address => uint256))` becomes:

```rust
// storage key: SCALE(keccak256("allowances") ++ keccak256(owner ++ spender))
// stored as: u128 little-endian
```

The `metadata.json` selectors:

```json
{
  "messages": [
    { "name": "transfer", "selector": "0xa9059cbb", "args": ["address","u128"], "ret": "bool", "mutates": true },
    { "name": "balanceOf", "selector": "0x70a08231", "args": ["address"], "ret": "u128", "mutates": false }
  ]
}
```

## Full lifecycle

```bash
inkport translate contracts/ERC20.sol
inkport build ERC20
inkport deploy ERC20 --arg 1000000
inkport call ERC20 balanceOf --arg //Alice   # → 1000000
inkport call ERC20 transfer --arg //Bob --arg 250
inkport call ERC20 balanceOf --arg //Bob     # → 250
```

## Test spec (`tests/ERC20.json`)

```json
{
  "deployer": "//Alice",
  "steps": [
    { "action": "deploy", "args": [1000000] },
    { "action": "read", "message": "balanceOf", "args": ["//Alice"], "expected": 1000000 },
    { "action": "call", "message": "transfer", "args": ["//Bob", 1000], "signer": "//Alice" },
    { "action": "event", "name": "Transfer", "expected": { "from": "//Alice", "to": "//Bob", "value": 1000 } },
    { "action": "read", "message": "balanceOf", "args": ["//Bob"], "expected": 1000 },
    { "action": "revert", "message": "transfer", "args": ["//Bob", 1000000000], "signer": "//Charlie" }
  ]
}
```
````

- [ ] **Step 2: Payable contracts guide**

`playground/app/(site)/docs/guides/payable/page.mdx`:
````mdx
export const metadata = { title: 'Payable Contracts — InkPort Guides' };

<div className="docs-tag">Guide</div>

# Payable contracts

Receiving and holding POT inside a contract.

## msg.value in Solidity → seal_value_transferred in seal0

When you mark a function `payable` in Solidity:

```solidity
function deposit() public payable {
    balances[msg.sender] += msg.value;
}
```

The translator emits a `seal_value_transferred` call that reads the transferred value in plancks (1 POT = 10¹⁴ plancks):

```rust
unsafe { seal0::seal_value_transferred(buf.as_mut_ptr(), &mut len) }
let amount = u128::from_le_bytes(buf);
```

## The Bank contract

```bash
inkport translate contracts/Bank.sol
inkport build Bank
inkport deploy Bank
inkport call Bank deposit --value 5          # deposit 5 POT
inkport call Bank balanceOf --arg //Alice    # → 500000000000000
inkport call Bank withdraw --arg 200000000000000
```

`--value 5` passes 5 POT (in plancks: `5 × 10¹⁴`). `balanceOf` returns plancks. Withdraw takes a planck amount.

## Test spec (`tests/Bank.json`)

```json
{
  "deployer": "//Alice",
  "steps": [
    { "action": "deploy" },
    { "action": "call", "message": "deposit", "value": 5, "signer": "//Alice" },
    { "action": "read", "message": "balanceOf", "args": ["//Alice"], "expected": 500000000000000 }
  ]
}
```

The `"value": 5` field in test steps is in POT; the harness converts to plancks before calling.
````

- [ ] **Step 3: Cross-contract calls guide**

`playground/app/(site)/docs/guides/cross-contract/page.mdx`:
````mdx
export const metadata = { title: 'Cross-Contract Calls — InkPort Guides' };

<div className="docs-tag">Guide</div>

# Cross-contract calls

Calling one deployed contract from another.

## Interface pattern in Solidity

```solidity
interface ITarget {
    function set(uint256 v) external;
    function get() external view returns (uint256);
}

contract Caller {
    ITarget private target;
    constructor(address t) { target = ITarget(t); }
    function callSet(uint256 v) public { target.set(v); }
    function callGet() public view returns (uint256) { return target.get(); }
}
```

The translator emits `seal_call` with the keccak4 selector of `set(u128)` / `get()`.

## Deploying the pair

The `@label` syntax in test specs lets you deploy `Target` first and reference its address in `Caller`'s constructor:

```json
{
  "deployer": "//Alice",
  "steps": [
    { "action": "deploy_dep", "name": "Target", "args": [], "as": "target" },
    { "action": "deploy", "args": ["@target"] },
    { "action": "call", "message": "callSet", "args": [42] },
    { "action": "read", "message": "callGet", "args": [], "expected": 42 }
  ]
}
```

`"@target"` is replaced at runtime with the SS58 address of the deployed `Target` instance.

## CLI

```bash
inkport deploy Target
# deployed Target -> 5DtarG...

inkport deploy Caller --arg 5DtarG...
inkport call Caller callSet --arg 42
inkport call Caller callGet              # → 42
```
````

- [ ] **Step 4: Integer width semantics guide**

`playground/app/(site)/docs/guides/integers/page.mdx`:
````mdx
export const metadata = { title: 'Integer Width Semantics — InkPort Guides' };

<div className="docs-tag">Guide</div>

# Integer width semantics

InkPort implements true bit-width semantics for every `uintN` / `intN` type.

## Width-checked arithmetic (default)

```solidity
function add(uint8 a, uint8 b) public pure returns (uint8) {
    return a + b;   // reverts if result > 255
}
```

`uint8 + uint8` is checked at the declared width. `255 + 1` reverts — it does **not** wrap silently.

## unchecked{} wraps

```solidity
function addWrap(uint8 a, uint8 b) public pure returns (uint8) {
    unchecked { return a + b; }   // 255 + 1 == 0
}
```

Inside `unchecked {}`, arithmetic wraps at the declared width. `uint8(255) + 1 == 0`.

## Narrowing casts truncate

```solidity
function cast8(uint256 v) public pure returns (uint8) {
    return uint8(v);   // uint8(256) == 0, uint8(257) == 1
}
```

Narrowing casts (`uint8(x)`) truncate to the lower N bits. `uint8(256) == 0`.

## All three behaviours verified on-chain

```bash
inkport test NarrowMath    # uint16 overflow → revert  ✓
inkport test Unchecked     # unchecked wraps            ✓
inkport test Cast          # uint8(256) == 0            ✓
```

## Storage representation

All integers are stored as `u128` / `i128` at rest (16-byte SCALE LE). Width enforcement happens at arithmetic boundaries, not in storage. A `uint8` variable reads back as a `u128` but arithmetic on it is checked at 255.
````

- [ ] **Step 5: Commit**

```bash
git add app/'(site)'/docs/guides/
git commit -m "feat(docs): feature guides — ERC20, payable, cross-contract, integers"
```

---

## Task 12: Reference + Troubleshooting docs

**Files:**
- `playground/app/(site)/docs/reference/metadata/page.mdx`
- `playground/app/(site)/docs/reference/test-spec/page.mdx`
- `playground/app/(site)/docs/reference/portaldot-node/page.mdx`
- `playground/app/(site)/docs/troubleshooting/page.mdx`

- [ ] **Step 1: metadata.json format**

`playground/app/(site)/docs/reference/metadata/page.mdx`:
````mdx
export const metadata = { title: 'metadata.json Format — InkPort Reference' };

<div className="docs-tag">Reference</div>

# metadata.json format

Emitted by `inkport translate`. Used by the CLI and harness to encode calls and decode returns.

## Schema

```json
{
  "name": "Counter",
  "constructor": {
    "args": ["u128"],
    "argNames": ["initial"]
  },
  "messages": [
    {
      "name": "inc",
      "selector": "0x371303c0",
      "args": [],
      "argNames": [],
      "ret": null,
      "mutates": true,
      "payable": false
    },
    {
      "name": "get",
      "selector": "0x6d4ce63c",
      "args": [],
      "argNames": [],
      "ret": "u128",
      "mutates": false,
      "payable": false
    }
  ],
  "events": [
    {
      "name": "Transfer",
      "fields": [
        { "name": "from", "type": "address" },
        { "name": "to",   "type": "address" },
        { "name": "value","type": "u128"    }
      ]
    }
  ]
}
```

## Fields

| Field | Type | Description |
|---|---|---|
| `name` | string | Contract name |
| `constructor.args` | string[] | SCALE types of constructor parameters |
| `constructor.argNames` | string[] | Parameter names (for CLI prompts) |
| `messages[].selector` | string | `0x` + first 4 bytes of `keccak256("name(canonicalTypes)")` |
| `messages[].args` | string[] | SCALE types of message parameters |
| `messages[].ret` | string \| null | SCALE type of return value, or null |
| `messages[].mutates` | boolean | `true` → submitted as extrinsic; `false` → dry-run read |
| `messages[].payable` | boolean | Whether the message accepts a value transfer |
| `events[].fields` | array | Field names and SCALE types for `ContractEmitted` decoding |

## Encoding

- Call input = `selector (4 bytes) ++ SCALE(arg0) ++ SCALE(arg1) ++ ...`
- Constructor input = `SCALE(arg0) ++ ...` (no selector)
- `u128` = 16-byte little-endian
- `bool` = 1 byte (`0x00` / `0x01`)
- `address` = 32 bytes (SS58 decoded to AccountId)
- `string`/`bytes` = compact-length prefix ++ UTF-8 bytes
````

- [ ] **Step 2: Test spec format**

`playground/app/(site)/docs/reference/test-spec/page.mdx`:
````mdx
export const metadata = { title: 'Test Spec Format — InkPort Reference' };

<div className="docs-tag">Reference</div>

# Test spec format

`tests/<Name>.json` drives `inkport test <Name>`. Each step is an action against the deployed contract.

## Top-level

```json
{
  "deployer": "//Alice",
  "steps": [ ... ]
}
```

| Field | Description |
|---|---|
| `deployer` | SURI used for the deploy step |
| `steps` | Ordered array of step objects |

## Action types

### deploy

```json
{ "action": "deploy", "args": [1000000] }
```

Calls `instantiate_with_code`. `args` = constructor arguments. Must be the first step.

### read

```json
{ "action": "read", "message": "balanceOf", "args": ["//Alice"], "expected": 1000000 }
```

Dry-run read. Asserts decoded return value equals `expected`.

### call

```json
{ "action": "call", "message": "transfer", "args": ["//Bob", 1000], "signer": "//Alice", "value": 0 }
```

Mutating extrinsic. `signer` defaults to `deployer`. `value` in POT for payable messages. Fails if the contract reverts.

### event

```json
{ "action": "event", "name": "Transfer", "expected": { "from": "//Alice", "to": "//Bob", "value": 1000 } }
```

Asserts the named event was emitted by the preceding `call`, with matching fields.

### revert

```json
{ "action": "revert", "message": "transfer", "args": ["//Bob", 1000000000], "signer": "//Charlie" }
```

Asserts the call reverts. Fails if it succeeds.

### deploy_dep

```json
{ "action": "deploy_dep", "name": "Target", "args": [], "as": "target" }
```

Deploy a helper contract. Reference its address in subsequent steps with `"@target"`.

## Address args

`"//Alice"`, `"//Bob"`, `"//Charlie"` resolve to their 32-byte AccountId. `"0x..."` hex strings are also accepted.
````

- [ ] **Step 3: Portaldot node reference**

`playground/app/(site)/docs/reference/portaldot-node/page.mdx`:
````mdx
export const metadata = { title: 'Portaldot Node — InkPort Reference' };

<div className="docs-tag">Reference</div>

# Portaldot node

| Field | Value |
|---|---|
| Public WSS | `wss://portaldot.philotheephilix.in` |
| Token | `POT` |
| Decimals | 14 (1 POT = 10¹⁴ plancks) |
| SS58 prefix | 42 |
| Pallet | rent-era `pallet-contracts` (seal0 ABI, ~Substrate 2021) |
| Faucet / Sudo | `//Alice` (`5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY`), pre-funded |
| Existential deposit | 1 POT |
| Default endowment | 10 POT (inkport deploy) |

## Connection requirements

Both parameters are **required** or `System.Account` won't decode:

```python
from substrateinterface import SubstrateInterface
substrate = SubstrateInterface(
    url="wss://portaldot.philotheephilix.in",
    ss58_format=42,
    type_registry_preset="substrate-node-template",
)
```

Omitting `type_registry_preset` causes `Decoder class for "AccountInfo..." not found`.

## Dev accounts

| Account | SURI | Address |
|---|---|---|
| Alice | `//Alice` | `5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY` |
| Bob | `//Bob` | `5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty` |
| Charlie | `//Charlie` | `5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y` |

> **Dev chain only.** No real value, state resets on node restart. Never reuse these keys anywhere with real value.
````

- [ ] **Step 4: Troubleshooting**

`playground/app/(site)/docs/troubleshooting/page.mdx`:
````mdx
export const metadata = { title: 'Troubleshooting — InkPort Docs' };

<div className="docs-tag">Troubleshooting</div>

# Troubleshooting

## System.Other on deploy

**Cause:** The node rejected the wasm. Two most common reasons:

1. **Bulk-memory instructions** (`memory.fill` / `memory.copy`) in the wasm. The codegen avoids these — if you wrote the Rust by hand, use `MaybeUninit` buffers and explicit byte loops. `-C target-feature=-bulk-memory` alone is not sufficient; the source must avoid the patterns.

2. **Oversized deploy buffer.** A large fixed input buffer (e.g. 512 bytes) makes `instantiate_with_code` fail. The codegen sizes buffers to the SCALE payload.

## Decoder class for "AccountInfo..." not found

**Cause:** Connected without `type_registry_preset='substrate-node-template'`.

**Fix:** The chain client sets this automatically. If you're using the SDK directly:

```python
SubstrateInterface(url=..., ss58_format=42, type_registry_preset="substrate-node-template")
```

## Dropped websocket mid-run

The chain client reconnects and retries on dropped sockets automatically. If a run still flakes, re-run `inkport test <Name>` — it deploys a fresh instance, so a previous partial run doesn't affect the result.

## inkport call prints "reverted"

This is correct behavior, not a bug. A view call that hits `require(false)` or an overflow at a narrow width prints `reverted`. Check whether your arguments satisfy the contract's preconditions.

## cargo contract / ink! errors

InkPort does not use `cargo-contract` or ink!. The build command is plain `cargo +stable build --target wasm32-unknown-unknown`. If you see ink!-related errors, make sure you're running `inkport build` and not `cargo contract build`.

## "Maximum number of pages should be always declared"

The wasm memory import must have a max declared. The linker flags in `onchain-contracts/counter/.cargo/config.toml` show the required settings:
```
--import-memory --initial-memory=65536 --max-memory=65536
```
The codegen emits these via the crate's `.cargo/config.toml`. If you're writing a contract by hand, copy that config.
````

- [ ] **Step 5: Commit**

```bash
git add app/'(site)'/docs/reference/ app/'(site)'/docs/troubleshooting/
git commit -m "feat(docs): reference (metadata, test-spec, portaldot-node) + troubleshooting"
```

---

## Self-review

**Spec coverage check:**
- `/home` landing page → Task 4 ✓
- `/why-inkport` → Task 5 ✓
- `/contracts` (30 cards + filter) → Task 6 ✓
- `/docs` hub with sidebar → Task 7 ✓
- Getting started (install, first-contract, project-layout) → Task 8 ✓
- CLI reference (6 commands) → Task 9 ✓
- Solidity coverage (supported + rejected) → Task 10 ✓
- Feature guides (ERC20, payable, cross-contract, integers) → Task 11 ✓
- Reference (metadata, test-spec, portaldot-node) + Troubleshooting → Task 12 ✓
- Shared nav + footer → Task 2 ✓
- `app/page.tsx` untouched → confirmed, no task modifies it ✓

**Placeholder scan:** No TBDs. All code blocks are complete.

**Type consistency:**
- `ContractEntry.tags` → `ContractTag[]` defined in Task 3, used in Task 6 ✓
- `DOCS_NAV` from `lib/docs-nav.ts` used in `DocsSidebar.tsx` ✓
- `Ic.bolt` imported from `@/components/Pipeline` in `SiteNav.tsx` ✓
- CSS classes in `site.css` match class names used in all page components ✓

**Parallel execution:**
- Tasks 4, 5, 6, 7 depend only on Tasks 1-3 → run in 4 parallel worktrees after Task 3 commits ✓
- Tasks 8-12 depend only on Task 7 → run in 5 parallel worktrees after Task 7 commits ✓
