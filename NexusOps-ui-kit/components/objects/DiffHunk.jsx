// DiffHunk — a reviewable code diff hunk with header, gutter, and an action bar.
// First-class review surface: accept / reject / ask / request-fix per hunk.
import React from 'react';

const LINE = {
  add: { bg: 'var(--diff-add-bg)', ink: 'var(--diff-add-ink)', gutter: 'var(--diff-add-gutter)', sign: '+' },
  del: { bg: 'var(--diff-del-bg)', ink: 'var(--diff-del-ink)', gutter: 'var(--diff-del-gutter)', sign: '-' },
  ctx: { bg: 'transparent', ink: 'var(--text-secondary)', gutter: 'transparent', sign: ' ' },
};

function HunkButton({ children, tone = 'default', onClick }) {
  const tones = {
    default: { c: 'var(--text-secondary)', b: 'var(--border-default)' },
    accept:  { c: 'var(--success-ink)', b: 'var(--success-line)' },
    reject:  { c: 'var(--danger-ink)', b: 'var(--danger-line)' },
    brain:   { c: 'var(--brain-ink)', b: 'var(--brain-line)' },
  };
  const t = tones[tone] || tones.default;
  return (
    <button type="button" onClick={onClick} style={{
      display: 'inline-flex', alignItems: 'center', gap: 4, height: 22, padding: '0 8px',
      borderRadius: 'var(--r-1)', border: '1px solid ' + t.b, background: 'transparent', color: t.c,
      font: 'var(--fw-medium) var(--fs-meta)/1 var(--font-sans)', cursor: 'pointer',
    }}>{children}</button>
  );
}

export function DiffHunk({
  file = 'file.ts', header = '@@ -1,4 +1,5 @@', lines = [],
  status,                 // undefined | 'accepted' | 'rejected' | 'conflict'
  comments = 0,
  actions = true,
  onAccept, onReject, onAsk, onRequestFix,
  style = {},
}) {
  const ribbon = status === 'accepted' ? 'var(--success-solid)'
    : status === 'rejected' ? 'var(--danger-solid)'
    : status === 'conflict' ? 'var(--critical-solid)' : 'transparent';

  return (
    <div style={{ border: '1px solid var(--border-default)', borderRadius: 'var(--r-2)', overflow: 'hidden', background: 'var(--surface-sunken)', ...style }}>
      {/* header */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '6px 10px', background: 'var(--surface-panel)', borderBottom: '1px solid var(--border-subtle)', borderLeft: '2px solid ' + ribbon }}>
        <span style={{ font: 'var(--fw-medium) var(--fs-meta)/1 var(--font-mono)', color: 'var(--text-primary)' }}>{file}</span>
        <span style={{ font: 'var(--fw-regular) var(--fs-meta)/1 var(--font-mono)', color: 'var(--text-faint)' }}>{header}</span>
        {comments > 0 && <span style={{ marginLeft: 'auto', font: '10px/1 var(--font-mono)', color: 'var(--brain-ink)' }}>{comments} ✦</span>}
        {status && <span style={{ marginLeft: comments ? 8 : 'auto', font: '10px/1 var(--font-sans)', fontWeight: 600, textTransform: 'uppercase', letterSpacing: '.05em', color: ribbon === 'transparent' ? 'var(--text-faint)' : ribbon }}>{status}</span>}
      </div>
      {/* code */}
      <div style={{ font: 'var(--fw-regular) var(--fs-meta)/1.6 var(--font-mono)', overflowX: 'auto' }}>
        {lines.map((l, i) => {
          const t = LINE[l.type] || LINE.ctx;
          return (
            <div key={i} style={{ display: 'flex', background: t.bg, boxShadow: t.gutter !== 'transparent' ? `inset 2px 0 0 ${t.gutter}` : 'none' }}>
              <span style={{ width: 30, textAlign: 'right', padding: '0 6px', color: 'var(--text-faint)', flex: 'none', userSelect: 'none' }}>{l.ln ?? ''}</span>
              <span style={{ width: 12, textAlign: 'center', color: t.ink, flex: 'none', userSelect: 'none' }}>{t.sign}</span>
              <span style={{ color: t.ink, whiteSpace: 'pre', paddingRight: 12 }}>{l.text}</span>
            </div>
          );
        })}
      </div>
      {/* action bar */}
      {actions && (
        <div style={{ display: 'flex', alignItems: 'center', gap: 6, padding: '6px 10px', borderTop: '1px solid var(--border-subtle)', background: 'var(--surface-panel)' }}>
          <HunkButton tone="accept" onClick={onAccept}>Accept</HunkButton>
          <HunkButton tone="reject" onClick={onReject}>Reject</HunkButton>
          <HunkButton tone="brain" onClick={onAsk}>Ask why</HunkButton>
          <HunkButton onClick={onRequestFix}>Request fix</HunkButton>
        </div>
      )}
    </div>
  );
}
