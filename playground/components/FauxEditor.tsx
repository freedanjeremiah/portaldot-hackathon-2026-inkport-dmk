'use client';
import { useRef } from 'react';
import { highlight } from '@/lib/highlight';

interface FauxEditorProps {
  value: string;
  onChange: (v: string) => void;
  language: 'solidity' | 'rust';
  readOnly: boolean;
}

export default function FauxEditor({ value, onChange, language, readOnly }: FauxEditorProps) {
  const taRef = useRef<HTMLTextAreaElement>(null);
  const preRef = useRef<HTMLPreElement>(null);
  const gutterRef = useRef<HTMLDivElement>(null);

  const lineCount = value.split('\n').length;
  const html = highlight(value.endsWith('\n') ? value + ' ' : value, language);

  const gutter: number[] = [];
  for (let i = 1; i <= lineCount; i++) gutter.push(i);

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Tab') {
      e.preventDefault();
      const ta = e.currentTarget;
      const s = ta.selectionStart, en = ta.selectionEnd;
      const nv = value.slice(0, s) + '    ' + value.slice(en);
      onChange(nv);
      requestAnimationFrame(() => { ta.selectionStart = ta.selectionEnd = s + 4; });
    }
  };

  if (readOnly) {
    return (
      <div className="feditor">
        <div className="gutter" ref={gutterRef}>
          {gutter.map(n => <div key={n} className="gln">{n}</div>)}
        </div>
        <div className="code-scroll" onScroll={e => { if (gutterRef.current) gutterRef.current.scrollTop = (e.target as HTMLElement).scrollTop; }}>
          <pre className="code-layer"><code dangerouslySetInnerHTML={{ __html: html }} /></pre>
        </div>
      </div>
    );
  }

  const onTaScroll = (e: React.UIEvent<HTMLTextAreaElement>) => {
    const st = e.currentTarget.scrollTop, sl = e.currentTarget.scrollLeft;
    if (preRef.current) preRef.current.style.transform = `translate(${-sl}px, ${-st}px)`;
    if (gutterRef.current) gutterRef.current.scrollTop = st;
  };

  return (
    <div className="feditor">
      <div className="gutter" ref={gutterRef}>
        {gutter.map(n => <div key={n} className="gln">{n}</div>)}
      </div>
      <div className="code-scroll" style={{ overflow: 'hidden' }}>
        <pre className="code-layer" ref={preRef} style={{ position: 'absolute', top: 0, left: 0, willChange: 'transform' }}>
          <code dangerouslySetInnerHTML={{ __html: html }} />
        </pre>
        <textarea
          ref={taRef}
          value={value}
          spellCheck={false}
          autoCapitalize="off"
          autoCorrect="off"
          autoComplete="off"
          onChange={e => onChange(e.target.value)}
          onScroll={onTaScroll}
          onKeyDown={handleKeyDown}
          style={{ overflow: 'auto' }}
        />
      </div>
    </div>
  );
}
