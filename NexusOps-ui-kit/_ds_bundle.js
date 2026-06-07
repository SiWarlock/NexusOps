/* @ds-bundle: {"format":3,"namespace":"ControlPlaneDesignSystem_a21911","components":[{"name":"Badge","sourcePath":"components/badges/Badge.jsx"},{"name":"HarnessBadge","sourcePath":"components/badges/HarnessBadge.jsx"},{"name":"MetaChip","sourcePath":"components/badges/MetaChip.jsx"},{"name":"ProfileBadge","sourcePath":"components/badges/ProfileBadge.jsx"},{"name":"Button","sourcePath":"components/controls/Button.jsx"},{"name":"IconButton","sourcePath":"components/controls/IconButton.jsx"},{"name":"DiffHunk","sourcePath":"components/objects/DiffHunk.jsx"},{"name":"EvidenceChip","sourcePath":"components/objects/EvidenceChip.jsx"},{"name":"GraphNode","sourcePath":"components/objects/GraphNode.jsx"},{"name":"SessionRow","sourcePath":"components/objects/SessionRow.jsx"},{"name":"AttentionMarker","sourcePath":"components/status/AttentionMarker.jsx"},{"name":"RiskBadge","sourcePath":"components/status/RiskBadge.jsx"},{"name":"STATUS","sourcePath":"components/status/StatusPill.jsx"},{"name":"StatusPill","sourcePath":"components/status/StatusPill.jsx"},{"name":"UsageMeter","sourcePath":"components/status/UsageMeter.jsx"}],"sourceHashes":{"components/badges/Badge.jsx":"bbf4081128d0","components/badges/HarnessBadge.jsx":"fb8d48711d68","components/badges/MetaChip.jsx":"6699aacdbe04","components/badges/ProfileBadge.jsx":"6549eabed5c1","components/controls/Button.jsx":"73d4d490837f","components/controls/IconButton.jsx":"b2414923f531","components/objects/DiffHunk.jsx":"092dfc8f7779","components/objects/EvidenceChip.jsx":"47117c27fc9e","components/objects/GraphNode.jsx":"0067d71ee677","components/objects/SessionRow.jsx":"7067efe7f16c","components/status/AttentionMarker.jsx":"849c381ddab3","components/status/RiskBadge.jsx":"f668e63430f4","components/status/StatusPill.jsx":"67314d2053dd","components/status/UsageMeter.jsx":"8428d679e20f","ui_kits/control-plane/kit-data.js":"efd3296f3527","ui_kits/control-plane/kit-overlays.jsx":"c4e379802bfc","ui_kits/control-plane/kit-plan.jsx":"db8f7710f9d0","ui_kits/control-plane/kit-shell.jsx":"450b83cd7ea1","ui_kits/control-plane/kit-tasks.jsx":"b188c1f72712","ui_kits/control-plane/kit-views.jsx":"213816ae884a","ui_kits/control-plane/kit-views2.jsx":"70fcfdb3adf7","ui_kits/control-plane/kit-views3.jsx":"449516d3c559","ui_kits/control-plane/kit-views4.jsx":"5ece5155dee9","ui_kits/control-plane/kit-views5.jsx":"71cf6fadb39b"},"inlinedExternals":[],"unexposedExports":[]} */

(() => {

const __ds_ns = (window.ControlPlaneDesignSystem_a21911 = window.ControlPlaneDesignSystem_a21911 || {});

const __ds_scope = {};

(__ds_ns.__errors = __ds_ns.__errors || []);

// components/badges/Badge.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
/**
 * Badge — small non-interactive label for counts, metadata, harness,
 * risk levels, and domain tags. Quieter than StatusPill (no glyph by default).
 */

const TONES = {
  neutral: ['--text-secondary', '--neutral-surface', '--border-default'],
  accent: ['--accent-ink', '--accent-surface', '--accent-line'],
  brain: ['--brain-ink', '--brain-surface', '--brain-line'],
  teal: ['--teal-ink', '--teal-surface', '--teal-line'],
  success: ['--success-ink', '--success-surface', '--success-line'],
  attention: ['--attention-ink', '--attention-surface', '--attention-line'],
  caution: ['--caution-ink', '--caution-surface', '--caution-line'],
  warning: ['--warning-ink', '--warning-surface', '--warning-line'],
  danger: ['--danger-ink', '--danger-surface', '--danger-line'],
  review: ['--review-ink', '--review-surface', '--review-line'],
  slate: ['--slate-ink', '--slate-surface', '--border-default']
};
function Badge({
  children,
  tone = 'neutral',
  variant = 'soft',
  // soft | solid | outline | dot
  mono = false,
  icon = null,
  size = 'sm',
  style = {},
  ...rest
}) {
  const [ink, surface, line] = TONES[tone] || TONES.neutral;
  const fs = {
    xs: '10px',
    sm: '11px',
    md: '12px'
  };
  const h = {
    xs: '16px',
    sm: '18px',
    md: 'var(--ctl-xs)'
  };
  let bg, fg, bd;
  if (variant === 'solid') {
    bg = `var(${ink.replace('-ink', '-solid')}, var(${ink}))`;
    fg = `var(${ink.replace('-ink', '-on-solid')}, var(--ink-on-solid))`;
    bd = 'transparent';
  } else if (variant === 'outline') {
    bg = 'transparent';
    fg = `var(${ink})`;
    bd = `var(${line})`;
  } else {
    bg = `var(${surface})`;
    fg = `var(${ink})`;
    bd = 'transparent';
  }
  return /*#__PURE__*/React.createElement("span", _extends({
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: '4px',
      height: h[size],
      padding: variant === 'dot' ? '0' : '0 6px',
      borderRadius: 'var(--r-1)',
      border: `1px solid ${variant === 'dot' ? 'transparent' : bd}`,
      background: variant === 'dot' ? 'transparent' : bg,
      color: fg,
      fontFamily: mono ? 'var(--font-mono)' : 'var(--font-sans)',
      fontSize: fs[size],
      fontWeight: 'var(--fw-medium)',
      letterSpacing: mono ? '0' : '0.01em',
      lineHeight: 1,
      whiteSpace: 'nowrap',
      flex: 'none',
      ...style
    }
  }, rest), variant === 'dot' && /*#__PURE__*/React.createElement("span", {
    style: {
      width: '6px',
      height: '6px',
      borderRadius: '50%',
      background: `var(${ink})`,
      flex: 'none'
    }
  }), icon && /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'inline-flex',
      flex: 'none'
    }
  }, icon), children);
}
Object.assign(__ds_scope, { Badge });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/badges/Badge.jsx", error: String((e && e.message) || e) }); }

// components/badges/HarnessBadge.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
// HarnessBadge — identifies the coding harness behind a session.
// Near-neutral by design (no official brand color implied); a faint
// warm/cool tint plus a glyph differentiates Claude vs Codex.

const HARNESS = {
  'claude-code': {
    label: 'Claude Code',
    tint: 'var(--domain-claude)',
    surf: 'var(--domain-claude-surface)',
    glyph: '✻'
  },
  'codex-cli': {
    label: 'Codex CLI',
    tint: 'var(--domain-codex)',
    surf: 'var(--domain-codex-surface)',
    glyph: '⌁'
  },
  'codex-cloud': {
    label: 'Codex Cloud',
    tint: 'var(--domain-codex)',
    surf: 'var(--domain-codex-surface)',
    glyph: '☁'
  },
  shell: {
    label: 'Custom Shell',
    tint: 'var(--harness-shell)',
    surf: 'var(--neutral-surface)',
    glyph: '$'
  }
};
function HarnessBadge({
  harness = 'claude-code',
  label,
  showLabel = true,
  style = {},
  ...rest
}) {
  const h = HARNESS[harness] || HARNESS['claude-code'];
  return /*#__PURE__*/React.createElement("span", _extends({
    title: h.label,
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 5,
      height: 'var(--ctl-xs)',
      padding: showLabel ? '0 7px 0 6px' : '0 5px',
      borderRadius: 'var(--r-1)',
      border: '1px solid var(--border-default)',
      background: h.surf,
      color: 'var(--text-secondary)',
      font: 'var(--fw-medium) var(--fs-meta)/1 var(--font-sans)',
      whiteSpace: 'nowrap',
      ...style
    }
  }, rest), /*#__PURE__*/React.createElement("span", {
    "aria-hidden": true,
    style: {
      fontFamily: 'var(--font-mono)',
      fontSize: 11,
      color: h.tint,
      lineHeight: 1
    }
  }, h.glyph), showLabel && (label || h.label));
}
Object.assign(__ds_scope, { HarnessBadge });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/badges/HarnessBadge.jsx", error: String((e && e.message) || e) }); }

// components/badges/MetaChip.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
// MetaChip — generic metadata chip (icon + monospace value). Used for
// branches, worktrees, models, paths, SHAs, ticket IDs, counts.

const TONES = {
  default: {
    ink: 'var(--text-secondary)',
    line: 'var(--border-default)',
    surf: 'transparent'
  },
  branch: {
    ink: 'var(--text-secondary)',
    line: 'var(--border-default)',
    surf: 'var(--surface-input)'
  },
  worktree: {
    ink: 'var(--teal-ink)',
    line: 'var(--teal-line)',
    surf: 'var(--teal-surface)'
  },
  pr: {
    ink: 'var(--review-ink)',
    line: 'var(--review-line)',
    surf: 'var(--review-surface)'
  },
  linear: {
    ink: 'var(--domain-linear)',
    line: 'var(--domain-linear-surface)',
    surf: 'var(--domain-linear-surface)'
  },
  github: {
    ink: 'var(--slate-ink)',
    line: 'var(--border-default)',
    surf: 'var(--slate-surface)'
  },
  brain: {
    ink: 'var(--brain-ink)',
    line: 'var(--brain-line)',
    surf: 'var(--brain-surface)'
  },
  accent: {
    ink: 'var(--accent-ink)',
    line: 'var(--accent-line)',
    surf: 'var(--accent-surface)'
  }
};
function MetaChip({
  children,
  icon = null,
  tone = 'default',
  mono = true,
  title,
  style = {},
  ...rest
}) {
  const t = TONES[tone] || TONES.default;
  return /*#__PURE__*/React.createElement("span", _extends({
    title: title,
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 4,
      maxWidth: '100%',
      height: 'var(--ctl-xs)',
      padding: '0 6px',
      borderRadius: 'var(--r-1)',
      border: '1px solid ' + t.line,
      background: t.surf,
      color: t.ink,
      font: `var(--fw-${mono ? 'regular' : 'medium'}) var(--fs-meta)/1 ${mono ? 'var(--font-mono)' : 'var(--font-sans)'}`,
      whiteSpace: 'nowrap',
      overflow: 'hidden',
      ...style
    }
  }, rest), icon && /*#__PURE__*/React.createElement("span", {
    "aria-hidden": true,
    style: {
      display: 'inline-flex',
      width: 12,
      height: 12,
      flex: 'none',
      opacity: 0.9
    }
  }, icon), /*#__PURE__*/React.createElement("span", {
    style: {
      overflow: 'hidden',
      textOverflow: 'ellipsis'
    }
  }, children));
}
Object.assign(__ds_scope, { MetaChip });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/badges/MetaChip.jsx", error: String((e && e.message) || e) }); }

// components/badges/ProfileBadge.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
// ProfileBadge — Execution Profile (account/runtime context) a session uses.
// Makes account routing explicit and auditable.

const HEALTH = {
  active: {
    dot: 'var(--success-solid)',
    label: 'active'
  },
  available: {
    dot: 'var(--slate-solid)',
    label: 'available'
  },
  'rate-limited': {
    dot: 'var(--warning-solid)',
    label: 'rate limited'
  },
  'auth-expired': {
    dot: 'var(--danger-solid)',
    label: 'auth expired'
  },
  disabled: {
    dot: 'var(--ink-4)',
    label: 'disabled'
  }
};
function ProfileBadge({
  name = 'Claude Max Main',
  provider = 'claude',
  health,
  size = 'sm',
  style = {},
  ...rest
}) {
  const h = health ? HEALTH[health] || HEALTH.available : null;
  const small = size === 'sm';
  return /*#__PURE__*/React.createElement("span", _extends({
    title: h ? `${name} · ${h.label}` : name,
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 6,
      height: small ? 'var(--ctl-xs)' : 'var(--ctl-sm)',
      padding: '0 8px',
      borderRadius: 'var(--r-1)',
      border: '1px solid var(--border-default)',
      background: 'var(--surface-input)',
      color: 'var(--text-secondary)',
      font: `var(--fw-medium) ${small ? 'var(--fs-meta)' : 'var(--fs-label)'}/1 var(--font-sans)`,
      whiteSpace: 'nowrap',
      ...style
    }
  }, rest), /*#__PURE__*/React.createElement("span", {
    "aria-hidden": true,
    style: {
      fontFamily: 'var(--font-mono)',
      fontSize: 10,
      color: 'var(--text-faint)',
      borderRight: '1px solid var(--border-subtle)',
      paddingRight: 6,
      lineHeight: 1.4,
      textTransform: 'uppercase'
    }
  }, provider === 'codex' ? 'CDX' : 'CLD'), /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-primary)'
    }
  }, name), h && /*#__PURE__*/React.createElement("span", {
    "aria-hidden": true,
    style: {
      width: 6,
      height: 6,
      borderRadius: '999px',
      background: h.dot,
      flex: 'none'
    }
  }));
}
Object.assign(__ds_scope, { ProfileBadge });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/badges/ProfileBadge.jsx", error: String((e && e.message) || e) }); }

// components/controls/Button.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
// Button — primary action control for the control plane.
// Self-contained: React only, styling via CSS custom properties.

const SIZES = {
  sm: {
    height: 'var(--ctl-sm)',
    padding: '0 10px',
    font: 'var(--fs-label)',
    radius: 'var(--r-2)',
    gap: '6px'
  },
  md: {
    height: 'var(--ctl-md)',
    padding: '0 12px',
    font: 'var(--fs-body)',
    radius: 'var(--r-2)',
    gap: '7px'
  },
  lg: {
    height: 'var(--ctl-lg)',
    padding: '0 16px',
    font: 'var(--fs-body-lg)',
    radius: 'var(--r-2)',
    gap: '8px'
  }
};
const VARIANTS = {
  primary: {
    background: 'var(--accent-solid)',
    color: 'var(--accent-on-solid)',
    border: '1px solid transparent'
  },
  secondary: {
    background: 'var(--surface-input)',
    color: 'var(--text-primary)',
    border: '1px solid var(--border-default)'
  },
  ghost: {
    background: 'transparent',
    color: 'var(--text-secondary)',
    border: '1px solid transparent'
  },
  outline: {
    background: 'transparent',
    color: 'var(--text-primary)',
    border: '1px solid var(--border-strong)'
  },
  danger: {
    background: 'var(--danger-solid)',
    color: 'var(--danger-on-solid)',
    border: '1px solid transparent'
  },
  brain: {
    background: 'var(--brain-surface)',
    color: 'var(--brain-ink)',
    border: '1px solid var(--brain-line)'
  }
};
function Button({
  children,
  variant = 'secondary',
  size = 'md',
  icon = null,
  iconRight = null,
  disabled = false,
  loading = false,
  full = false,
  kbd = null,
  onClick,
  style = {},
  ...rest
}) {
  const s = SIZES[size] || SIZES.md;
  const v = VARIANTS[variant] || VARIANTS.secondary;
  const isDisabled = disabled || loading;
  return /*#__PURE__*/React.createElement("button", _extends({
    type: "button",
    disabled: isDisabled,
    onClick: onClick,
    style: {
      display: full ? 'flex' : 'inline-flex',
      width: full ? '100%' : 'auto',
      alignItems: 'center',
      justifyContent: 'center',
      gap: s.gap,
      height: s.height,
      padding: s.padding,
      borderRadius: s.radius,
      font: `var(--fw-medium) ${s.font}/1 var(--font-sans)`,
      letterSpacing: '0.005em',
      cursor: isDisabled ? 'not-allowed' : 'pointer',
      opacity: isDisabled ? 0.45 : 1,
      whiteSpace: 'nowrap',
      userSelect: 'none',
      transition: 'background var(--dur-1) var(--ease-standard), border-color var(--dur-1), filter var(--dur-1), transform var(--dur-1)',
      ...v,
      ...style
    },
    onMouseDown: e => {
      if (!isDisabled) e.currentTarget.style.transform = 'scale(var(--press-scale))';
    },
    onMouseUp: e => {
      e.currentTarget.style.transform = 'scale(1)';
    },
    onMouseLeave: e => {
      e.currentTarget.style.transform = 'scale(1)';
    }
  }, rest), loading ? /*#__PURE__*/React.createElement("span", {
    "aria-hidden": true,
    style: {
      width: 12,
      height: 12,
      borderRadius: '999px',
      border: '1.5px solid currentColor',
      borderTopColor: 'transparent',
      animation: 'cp-live-pulse 1s linear infinite',
      opacity: 0.8
    }
  }) : icon, children, iconRight, kbd && /*#__PURE__*/React.createElement("kbd", {
    style: {
      marginLeft: 4,
      padding: '1px 5px',
      borderRadius: 'var(--r-1)',
      background: 'oklch(1 0 0 / 0.10)',
      border: '1px solid oklch(1 0 0 / 0.12)',
      font: 'var(--fw-medium) 10px/1 var(--font-mono)',
      opacity: 0.85
    }
  }, kbd));
}
Object.assign(__ds_scope, { Button });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/controls/Button.jsx", error: String((e && e.message) || e) }); }

// components/controls/IconButton.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
// IconButton — square icon-only control for dense toolbars and rows.

const SIZES = {
  sm: 22,
  md: 26,
  lg: 30
};
function IconButton({
  children,
  label,
  size = 'md',
  variant = 'ghost',
  active = false,
  disabled = false,
  badge = null,
  onClick,
  style = {},
  ...rest
}) {
  const dim = SIZES[size] || SIZES.md;
  const base = {
    ghost: {
      background: active ? 'var(--surface-active)' : 'transparent',
      color: active ? 'var(--accent-ink)' : 'var(--text-secondary)',
      border: '1px solid ' + (active ? 'var(--accent-line)' : 'transparent')
    },
    solid: {
      background: 'var(--surface-input)',
      color: 'var(--text-primary)',
      border: '1px solid var(--border-default)'
    },
    danger: {
      background: 'transparent',
      color: 'var(--danger-ink)',
      border: '1px solid transparent'
    }
  }[variant] || {};
  return /*#__PURE__*/React.createElement("button", _extends({
    type: "button",
    "aria-label": label,
    title: label,
    disabled: disabled,
    onClick: onClick,
    style: {
      position: 'relative',
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      width: dim,
      height: dim,
      flex: 'none',
      borderRadius: 'var(--r-2)',
      cursor: disabled ? 'not-allowed' : 'pointer',
      opacity: disabled ? 0.4 : 1,
      transition: 'background var(--dur-1) var(--ease-standard), color var(--dur-1), border-color var(--dur-1)',
      ...base,
      ...style
    },
    onMouseEnter: e => {
      if (!disabled && !active && variant === 'ghost') {
        e.currentTarget.style.background = 'var(--surface-hover)';
        e.currentTarget.style.color = 'var(--text-primary)';
      }
    },
    onMouseLeave: e => {
      if (!active && variant === 'ghost') {
        e.currentTarget.style.background = 'transparent';
        e.currentTarget.style.color = 'var(--text-secondary)';
      }
    }
  }, rest), /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'inline-flex',
      width: dim <= 22 ? 14 : 16,
      height: dim <= 22 ? 14 : 16
    }
  }, children), badge != null && /*#__PURE__*/React.createElement("span", {
    style: {
      position: 'absolute',
      top: -4,
      right: -4,
      minWidth: 14,
      height: 14,
      padding: '0 3px',
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      borderRadius: '999px',
      background: 'var(--attention-solid)',
      color: 'var(--attention-on-solid)',
      font: 'var(--fw-semibold) 9px/1 var(--font-mono)',
      border: '1.5px solid var(--surface-panel)'
    }
  }, badge));
}
Object.assign(__ds_scope, { IconButton });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/controls/IconButton.jsx", error: String((e && e.message) || e) }); }

// components/objects/DiffHunk.jsx
try { (() => {
// DiffHunk — a reviewable code diff hunk with header, gutter, and an action bar.
// First-class review surface: accept / reject / ask / request-fix per hunk.

const LINE = {
  add: {
    bg: 'var(--diff-add-bg)',
    ink: 'var(--diff-add-ink)',
    gutter: 'var(--diff-add-gutter)',
    sign: '+'
  },
  del: {
    bg: 'var(--diff-del-bg)',
    ink: 'var(--diff-del-ink)',
    gutter: 'var(--diff-del-gutter)',
    sign: '-'
  },
  ctx: {
    bg: 'transparent',
    ink: 'var(--text-secondary)',
    gutter: 'transparent',
    sign: ' '
  }
};
function HunkButton({
  children,
  tone = 'default',
  onClick
}) {
  const tones = {
    default: {
      c: 'var(--text-secondary)',
      b: 'var(--border-default)'
    },
    accept: {
      c: 'var(--success-ink)',
      b: 'var(--success-line)'
    },
    reject: {
      c: 'var(--danger-ink)',
      b: 'var(--danger-line)'
    },
    brain: {
      c: 'var(--brain-ink)',
      b: 'var(--brain-line)'
    }
  };
  const t = tones[tone] || tones.default;
  return /*#__PURE__*/React.createElement("button", {
    type: "button",
    onClick: onClick,
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 4,
      height: 22,
      padding: '0 8px',
      borderRadius: 'var(--r-1)',
      border: '1px solid ' + t.b,
      background: 'transparent',
      color: t.c,
      font: 'var(--fw-medium) var(--fs-meta)/1 var(--font-sans)',
      cursor: 'pointer'
    }
  }, children);
}
function DiffHunk({
  file = 'file.ts',
  header = '@@ -1,4 +1,5 @@',
  lines = [],
  status,
  // undefined | 'accepted' | 'rejected' | 'conflict'
  comments = 0,
  actions = true,
  onAccept,
  onReject,
  onAsk,
  onRequestFix,
  style = {}
}) {
  const ribbon = status === 'accepted' ? 'var(--success-solid)' : status === 'rejected' ? 'var(--danger-solid)' : status === 'conflict' ? 'var(--critical-solid)' : 'transparent';
  return /*#__PURE__*/React.createElement("div", {
    style: {
      border: '1px solid var(--border-default)',
      borderRadius: 'var(--r-2)',
      overflow: 'hidden',
      background: 'var(--surface-sunken)',
      ...style
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      padding: '6px 10px',
      background: 'var(--surface-panel)',
      borderBottom: '1px solid var(--border-subtle)',
      borderLeft: '2px solid ' + ribbon
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fw-medium) var(--fs-meta)/1 var(--font-mono)',
      color: 'var(--text-primary)'
    }
  }, file), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fw-regular) var(--fs-meta)/1 var(--font-mono)',
      color: 'var(--text-faint)'
    }
  }, header), comments > 0 && /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: 'auto',
      font: '10px/1 var(--font-mono)',
      color: 'var(--brain-ink)'
    }
  }, comments, " \u2726"), status && /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: comments ? 8 : 'auto',
      font: '10px/1 var(--font-sans)',
      fontWeight: 600,
      textTransform: 'uppercase',
      letterSpacing: '.05em',
      color: ribbon === 'transparent' ? 'var(--text-faint)' : ribbon
    }
  }, status)), /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fw-regular) var(--fs-meta)/1.6 var(--font-mono)',
      overflowX: 'auto'
    }
  }, lines.map((l, i) => {
    const t = LINE[l.type] || LINE.ctx;
    return /*#__PURE__*/React.createElement("div", {
      key: i,
      style: {
        display: 'flex',
        background: t.bg,
        boxShadow: t.gutter !== 'transparent' ? `inset 2px 0 0 ${t.gutter}` : 'none'
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        width: 30,
        textAlign: 'right',
        padding: '0 6px',
        color: 'var(--text-faint)',
        flex: 'none',
        userSelect: 'none'
      }
    }, l.ln ?? ''), /*#__PURE__*/React.createElement("span", {
      style: {
        width: 12,
        textAlign: 'center',
        color: t.ink,
        flex: 'none',
        userSelect: 'none'
      }
    }, t.sign), /*#__PURE__*/React.createElement("span", {
      style: {
        color: t.ink,
        whiteSpace: 'pre',
        paddingRight: 12
      }
    }, l.text));
  })), actions && /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 6,
      padding: '6px 10px',
      borderTop: '1px solid var(--border-subtle)',
      background: 'var(--surface-panel)'
    }
  }, /*#__PURE__*/React.createElement(HunkButton, {
    tone: "accept",
    onClick: onAccept
  }, "Accept"), /*#__PURE__*/React.createElement(HunkButton, {
    tone: "reject",
    onClick: onReject
  }, "Reject"), /*#__PURE__*/React.createElement(HunkButton, {
    tone: "brain",
    onClick: onAsk
  }, "Ask why"), /*#__PURE__*/React.createElement(HunkButton, {
    onClick: onRequestFix
  }, "Request fix")));
}
Object.assign(__ds_scope, { DiffHunk });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/objects/DiffHunk.jsx", error: String((e && e.message) || e) }); }

// components/objects/EvidenceChip.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
// EvidenceChip — a Project Brain evidence reference. Grounds every Brain
// answer/action in a real object the user can open. Violet identity.

const KIND = {
  file: {
    glyph: '⌗',
    label: 'file'
  },
  anchor: {
    glyph: '⚓',
    label: 'anchor'
  },
  plantask: {
    glyph: '◇',
    label: 'plan'
  },
  session: {
    glyph: '▷',
    label: 'session'
  },
  commit: {
    glyph: '⎇',
    label: 'commit'
  },
  pr: {
    glyph: '⇡',
    label: 'PR'
  },
  decision: {
    glyph: '§',
    label: 'decision'
  },
  ticket: {
    glyph: '#',
    label: 'ticket'
  },
  event: {
    glyph: '◴',
    label: 'event'
  },
  memory: {
    glyph: '✻',
    label: 'memory'
  }
};
function EvidenceChip({
  kind = 'file',
  label,
  sub,
  freshness,
  onClick,
  style = {},
  ...rest
}) {
  const k = KIND[kind] || KIND.file;
  const stale = freshness === 'stale';
  return /*#__PURE__*/React.createElement("button", _extends({
    type: "button",
    onClick: onClick,
    title: sub ? `${label} — ${sub}` : label,
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 6,
      maxWidth: '100%',
      height: 'var(--ctl-sm)',
      padding: '0 8px 0 6px',
      borderRadius: 'var(--r-1)',
      border: '1px solid var(--brain-line)',
      background: 'var(--brain-surface)',
      color: 'var(--brain-ink)',
      cursor: 'pointer',
      textAlign: 'left',
      font: 'var(--fw-regular) var(--fs-meta)/1 var(--font-sans)',
      whiteSpace: 'nowrap',
      transition: 'background var(--dur-1) var(--ease-standard)',
      ...style
    },
    onMouseEnter: e => {
      e.currentTarget.style.background = 'var(--brain-surface-2)';
    },
    onMouseLeave: e => {
      e.currentTarget.style.background = 'var(--brain-surface)';
    }
  }, rest), /*#__PURE__*/React.createElement("span", {
    "aria-hidden": true,
    style: {
      fontFamily: 'var(--font-mono)',
      fontSize: 11,
      lineHeight: 1,
      color: 'var(--brain-bright)'
    }
  }, k.glyph), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-mono)',
      color: 'var(--text-primary)',
      overflow: 'hidden',
      textOverflow: 'ellipsis'
    }
  }, label), sub && /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-faint)',
      overflow: 'hidden',
      textOverflow: 'ellipsis'
    }
  }, sub), stale && /*#__PURE__*/React.createElement("span", {
    "aria-label": "stale",
    title: "stale",
    style: {
      width: 6,
      height: 6,
      borderRadius: '999px',
      background: 'var(--warning-solid)',
      flex: 'none'
    }
  }));
}
Object.assign(__ds_scope, { EvidenceChip });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/objects/EvidenceChip.jsx", error: String((e && e.message) || e) }); }

// components/objects/GraphNode.jsx
try { (() => {
// GraphNode — an operational node for the observability graph. NOT decorative:
// it shows node type (glyph + domain tint), status, ownership, and exposes a
// selected/attention state. Node chrome = domain; status ring = state.

const KIND = {
  project: {
    glyph: '◰',
    tint: 'var(--text-secondary)',
    label: 'Project'
  },
  session: {
    glyph: '▷',
    tint: 'var(--accent-ink)',
    label: 'Session'
  },
  team: {
    glyph: '⧉',
    tint: 'var(--teal-ink)',
    label: 'Agent Team'
  },
  worker: {
    glyph: '▸',
    tint: 'var(--teal-ink)',
    label: 'Worker'
  },
  worktree: {
    glyph: '⌥',
    tint: 'var(--teal-ink)',
    label: 'Worktree'
  },
  branch: {
    glyph: '⎇',
    tint: 'var(--slate-ink)',
    label: 'Branch'
  },
  pr: {
    glyph: '⇡',
    tint: 'var(--review-ink)',
    label: 'Pull Request'
  },
  issue: {
    glyph: '#',
    tint: 'var(--slate-ink)',
    label: 'GitHub Issue'
  },
  ticket: {
    glyph: '◇',
    tint: 'var(--domain-linear)',
    label: 'Linear Ticket'
  },
  plantask: {
    glyph: '◇',
    tint: 'var(--accent-ink)',
    label: 'Plan Task'
  },
  approval: {
    glyph: '⊘',
    tint: 'var(--caution-ink)',
    label: 'Approval'
  },
  human: {
    glyph: '◆',
    tint: 'var(--attention-ink)',
    label: 'Human input'
  },
  brain: {
    glyph: '✻',
    tint: 'var(--brain-ink)',
    label: 'Project Brain'
  }
};

// status -> ring color
const RING = {
  active: 'var(--accent-solid)',
  running: 'var(--live-solid)',
  'waiting-human': 'var(--attention-solid)',
  'waiting-perm': 'var(--caution-solid)',
  failed: 'var(--danger-solid)',
  blocked: 'var(--danger-solid)',
  conflict: 'var(--danger-solid)',
  stale: 'var(--warning-solid)',
  degraded: 'var(--warning-solid)',
  completed: 'var(--success-solid)',
  idle: 'var(--border-strong)',
  'pr-open': 'var(--review-solid)'
};
function GraphNode({
  kind = 'session',
  title = 'Node',
  subtitle,
  status,
  owner,
  meta = [],
  selected = false,
  beacon = false,
  onClick,
  style = {}
}) {
  const k = KIND[kind] || KIND.session;
  const ring = status ? RING[status] || 'var(--border-strong)' : 'var(--border-strong)';
  return /*#__PURE__*/React.createElement("div", {
    onClick: onClick,
    role: "button",
    "aria-pressed": selected,
    style: {
      position: 'relative',
      display: 'flex',
      flexDirection: 'column',
      gap: 5,
      width: 196,
      padding: '9px 11px',
      cursor: 'pointer',
      background: 'var(--graph-node-bg)',
      borderRadius: 'var(--r-3)',
      border: '1px solid ' + (selected ? 'var(--graph-selected)' : 'var(--graph-node-line)'),
      boxShadow: selected ? 'var(--graph-selected-glow)' : 'var(--elev-1)',
      outline: selected ? '1px solid var(--accent-line)' : 'none',
      transition: 'border-color var(--dur-1), box-shadow var(--dur-2)',
      ...style
    },
    onMouseEnter: e => {
      if (!selected) e.currentTarget.style.borderColor = 'var(--graph-node-line-strong)';
    },
    onMouseLeave: e => {
      if (!selected) e.currentTarget.style.borderColor = 'var(--graph-node-line)';
    }
  }, status && /*#__PURE__*/React.createElement("span", {
    "aria-label": status,
    title: status,
    style: {
      position: 'absolute',
      top: 9,
      right: 9,
      width: 9,
      height: 9,
      borderRadius: '999px',
      background: ring,
      boxShadow: beacon ? 'var(--attention-glow)' : 'none',
      animation: status === 'running' && 'cp-live-pulse 1.6s var(--ease-inout) infinite' || beacon && 'cp-attention-beacon 2.2s var(--ease-out) infinite' || 'none'
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 7,
      paddingRight: 14
    }
  }, /*#__PURE__*/React.createElement("span", {
    "aria-hidden": true,
    style: {
      fontFamily: 'var(--font-mono)',
      fontSize: 13,
      color: k.tint,
      lineHeight: 1,
      flex: 'none'
    }
  }, k.glyph), /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      minWidth: 0
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fw-semibold) 9px/1 var(--font-sans)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: 'var(--text-faint)'
    }
  }, k.label), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fw-medium) var(--fs-label)/1.25 var(--font-sans)',
      color: 'var(--text-primary)',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap'
    }
  }, title))), subtitle && /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fw-regular) var(--fs-meta)/1.3 var(--font-mono)',
      color: 'var(--text-muted)',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap'
    }
  }, subtitle), (owner || meta.length > 0) && /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 5,
      flexWrap: 'wrap'
    }
  }, owner && /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fw-medium) 10px/1 var(--font-sans)',
      color: 'var(--text-faint)'
    }
  }, owner), meta.map((m, i) => /*#__PURE__*/React.createElement("span", {
    key: i,
    style: {
      padding: '1px 5px',
      borderRadius: 'var(--r-1)',
      border: '1px solid var(--border-subtle)',
      background: 'var(--surface-input)',
      font: '10px/1.4 var(--font-mono)',
      color: 'var(--text-muted)'
    }
  }, m))));
}
Object.assign(__ds_scope, { GraphNode });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/objects/GraphNode.jsx", error: String((e && e.message) || e) }); }

// components/status/AttentionMarker.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
// AttentionMarker — the left rail / dot that encodes a row's attention level.
// Drives the visual weight of sidebar rows, queue items, and graph nodes.

const LEVELS = {
  5: {
    color: 'var(--attention-solid)',
    beacon: true,
    label: 'Waiting on human'
  },
  4: {
    color: 'var(--danger-solid)',
    beacon: false,
    label: 'Failed / blocked'
  },
  3: {
    color: 'var(--warning-solid)',
    beacon: false,
    label: 'Degraded / capacity'
  },
  2: {
    color: 'var(--live-solid)',
    pulse: true,
    label: 'Running'
  },
  1: {
    color: 'var(--accent-solid)',
    beacon: false,
    label: 'Active'
  },
  0: {
    color: 'var(--slate-solid)',
    beacon: false,
    label: 'Idle'
  }
};
function AttentionMarker({
  level = 0,
  variant = 'rail',
  style = {},
  ...rest
}) {
  const l = LEVELS[level] || LEVELS[0];
  const animate = l.beacon ? 'cp-attention-beacon 2.2s var(--ease-out) infinite' : l.pulse ? 'cp-live-pulse 1.6s var(--ease-inout) infinite' : 'none';
  if (variant === 'dot') {
    return /*#__PURE__*/React.createElement("span", _extends({
      role: "img",
      "aria-label": l.label,
      title: l.label,
      style: {
        width: 8,
        height: 8,
        borderRadius: '999px',
        background: l.color,
        flex: 'none',
        animation: animate,
        ...style
      }
    }, rest));
  }
  // rail: a vertical bar meant to sit at the leading edge of a row/card
  return /*#__PURE__*/React.createElement("span", _extends({
    "aria-label": l.label,
    title: l.label,
    style: {
      width: level >= 4 ? 3 : level >= 1 ? 2 : 0,
      alignSelf: 'stretch',
      borderRadius: '0 2px 2px 0',
      background: level === 0 ? 'transparent' : l.color,
      flex: 'none',
      animation: l.pulse ? animate : 'none',
      ...style
    }
  }, rest));
}
Object.assign(__ds_scope, { AttentionMarker });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/status/AttentionMarker.jsx", error: String((e && e.message) || e) }); }

// components/status/RiskBadge.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
// RiskBadge — Action Gateway risk classification. Text label is mandatory;
// critical adds a hazard hatch so it reads in grayscale.

const RISK = {
  readonly: {
    ink: 'var(--risk-readonly)',
    line: 'var(--border-default)',
    label: 'Read-only'
  },
  low: {
    ink: 'var(--risk-low)',
    line: 'var(--success-line)',
    label: 'Low'
  },
  medium: {
    ink: 'var(--risk-medium)',
    line: 'var(--caution-line)',
    label: 'Medium'
  },
  high: {
    ink: 'var(--risk-high)',
    line: 'oklch(0.70 0.18 38 / 0.45)',
    label: 'High'
  },
  critical: {
    ink: 'var(--risk-critical)',
    line: 'var(--critical-line)',
    label: 'Critical'
  }
};
function RiskBadge({
  level = 'low',
  label,
  showDot = true,
  style = {},
  ...rest
}) {
  const r = RISK[level] || RISK.low;
  const isCritical = level === 'critical';
  return /*#__PURE__*/React.createElement("span", _extends({
    className: isCritical ? 'cp-hazard' : undefined,
    title: `Risk: ${label || r.label}`,
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 5,
      height: 'var(--ctl-xs)',
      padding: '0 7px',
      borderRadius: 'var(--r-1)',
      font: 'var(--fw-semibold) var(--fs-micro)/1 var(--font-sans)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: r.ink,
      border: '1px solid ' + r.line,
      background: isCritical ? undefined : 'transparent',
      ...style
    }
  }, rest), showDot && /*#__PURE__*/React.createElement("span", {
    "aria-hidden": true,
    style: {
      width: 6,
      height: 6,
      borderRadius: '2px',
      background: r.ink,
      flex: 'none'
    }
  }), label || r.label);
}
Object.assign(__ds_scope, { RiskBadge });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/status/RiskBadge.jsx", error: String((e && e.message) || e) }); }

// components/status/StatusPill.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
// StatusPill — the canonical status marker. Encodes state on FOUR
// channels: color family, glyph, text label, and (optional) motion.

// status -> { ink, surface, line, solid, onSolid, glyph, label, beacon }
const STATUS = {
  active: {
    ink: 'var(--accent-ink)',
    surface: 'var(--accent-surface)',
    line: 'var(--accent-line)',
    solid: 'var(--accent-solid)',
    onSolid: 'var(--accent-on-solid)',
    glyph: '●',
    label: 'Active'
  },
  running: {
    ink: 'var(--live-ink)',
    surface: 'var(--live-surface)',
    line: 'var(--live-line)',
    solid: 'var(--live-solid)',
    onSolid: 'var(--live-on-solid)',
    glyph: '▶',
    label: 'Running',
    pulse: true
  },
  editing: {
    ink: 'var(--accent-ink)',
    surface: 'var(--accent-surface)',
    line: 'var(--accent-line)',
    solid: 'var(--accent-solid)',
    onSolid: 'var(--accent-on-solid)',
    glyph: '✎',
    label: 'Editing',
    pulse: true
  },
  testing: {
    ink: 'var(--live-ink)',
    surface: 'var(--live-surface)',
    line: 'var(--live-line)',
    solid: 'var(--live-solid)',
    onSolid: 'var(--live-on-solid)',
    glyph: '◑',
    label: 'Running tests',
    pulse: true
  },
  idle: {
    ink: 'var(--slate-ink)',
    surface: 'var(--slate-surface)',
    line: 'var(--border-default)',
    solid: 'var(--slate-solid)',
    onSolid: 'var(--slate-on-solid)',
    glyph: '○',
    label: 'Idle'
  },
  'waiting-human': {
    ink: 'var(--attention-ink)',
    surface: 'var(--attention-surface)',
    line: 'var(--attention-line)',
    solid: 'var(--attention-solid)',
    onSolid: 'var(--attention-on-solid)',
    glyph: '◆',
    label: 'Waiting · human',
    beacon: true
  },
  'waiting-perm': {
    ink: 'var(--caution-ink)',
    surface: 'var(--caution-surface)',
    line: 'var(--caution-line)',
    solid: 'var(--caution-solid)',
    onSolid: 'var(--caution-on-solid)',
    glyph: '⊘',
    label: 'Waiting · permission'
  },
  approval: {
    ink: 'var(--attention-ink)',
    surface: 'var(--attention-surface)',
    line: 'var(--attention-line)',
    solid: 'var(--attention-solid)',
    onSolid: 'var(--attention-on-solid)',
    glyph: '◆',
    label: 'Approval required',
    beacon: true
  },
  failed: {
    ink: 'var(--danger-ink)',
    surface: 'var(--danger-surface)',
    line: 'var(--danger-line)',
    solid: 'var(--danger-solid)',
    onSolid: 'var(--danger-on-solid)',
    glyph: '✕',
    label: 'Failed'
  },
  blocked: {
    ink: 'var(--danger-ink)',
    surface: 'var(--danger-surface)',
    line: 'var(--danger-line)',
    solid: 'var(--danger-solid)',
    onSolid: 'var(--danger-on-solid)',
    glyph: '■',
    label: 'Blocked'
  },
  conflict: {
    ink: 'var(--danger-ink)',
    surface: 'var(--danger-surface)',
    line: 'var(--danger-line)',
    solid: 'var(--danger-solid)',
    onSolid: 'var(--danger-on-solid)',
    glyph: '⨯',
    label: 'Conflict'
  },
  stale: {
    ink: 'var(--warning-ink)',
    surface: 'var(--warning-surface)',
    line: 'var(--warning-line)',
    solid: 'var(--warning-solid)',
    onSolid: 'var(--warning-on-solid)',
    glyph: '◌',
    label: 'Stale'
  },
  degraded: {
    ink: 'var(--warning-ink)',
    surface: 'var(--warning-surface)',
    line: 'var(--warning-line)',
    solid: 'var(--warning-solid)',
    onSolid: 'var(--warning-on-solid)',
    glyph: '△',
    label: 'Degraded'
  },
  completed: {
    ink: 'var(--success-ink)',
    surface: 'var(--success-surface)',
    line: 'var(--success-line)',
    solid: 'var(--success-solid)',
    onSolid: 'var(--success-on-solid)',
    glyph: '✓',
    label: 'Completed'
  },
  approved: {
    ink: 'var(--success-ink)',
    surface: 'var(--success-surface)',
    line: 'var(--success-line)',
    solid: 'var(--success-solid)',
    onSolid: 'var(--success-on-solid)',
    glyph: '✓',
    label: 'Approved'
  },
  passing: {
    ink: 'var(--success-ink)',
    surface: 'var(--success-surface)',
    line: 'var(--success-line)',
    solid: 'var(--success-solid)',
    onSolid: 'var(--success-on-solid)',
    glyph: '✓',
    label: 'Checks passing'
  },
  archived: {
    ink: 'var(--ink-4)',
    surface: 'var(--slate-surface)',
    line: 'var(--border-subtle)',
    solid: 'var(--slate-solid)',
    onSolid: 'var(--slate-on-solid)',
    glyph: '—',
    label: 'Archived'
  },
  'pr-open': {
    ink: 'var(--review-ink)',
    surface: 'var(--review-surface)',
    line: 'var(--review-line)',
    solid: 'var(--review-solid)',
    onSolid: 'var(--review-on-solid)',
    glyph: '⇡',
    label: 'PR open'
  },
  merged: {
    ink: 'var(--review-ink)',
    surface: 'var(--review-surface)',
    line: 'var(--review-line)',
    solid: 'var(--review-solid)',
    onSolid: 'var(--review-on-solid)',
    glyph: '⇊',
    label: 'Merged'
  },
  critical: {
    ink: 'var(--critical-ink)',
    surface: 'var(--critical-surface)',
    line: 'var(--critical-line)',
    solid: 'var(--critical-solid)',
    onSolid: 'var(--critical-on-solid)',
    glyph: '!',
    label: 'Critical'
  }
};
const SIZES = {
  xs: {
    h: 'var(--ctl-xs)',
    pad: '0 6px',
    font: 'var(--fs-micro)',
    glyph: 8
  },
  sm: {
    h: 'var(--ctl-xs)',
    pad: '0 7px',
    font: 'var(--fs-meta)',
    glyph: 9
  },
  md: {
    h: 'var(--ctl-sm)',
    pad: '0 9px',
    font: 'var(--fs-label)',
    glyph: 10
  }
};
function StatusPill({
  status = 'idle',
  label,
  emphasis = 'soft',
  size = 'sm',
  beacon,
  style = {},
  ...rest
}) {
  const s = STATUS[status] || STATUS.idle;
  const sz = SIZES[size] || SIZES.sm;
  const solid = emphasis === 'solid';
  const showBeacon = beacon != null ? beacon : s.beacon;
  return /*#__PURE__*/React.createElement("span", _extends({
    role: "status",
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 5,
      height: sz.h,
      padding: sz.pad,
      borderRadius: 'var(--r-1)',
      font: `var(--fw-medium) ${sz.font}/1 var(--font-sans)`,
      letterSpacing: 'var(--tracking-wide)',
      whiteSpace: 'nowrap',
      background: solid ? s.solid : s.surface,
      color: solid ? s.onSolid : s.ink,
      border: '1px solid ' + (solid ? 'transparent' : s.line),
      boxShadow: showBeacon && solid ? 'var(--attention-glow)' : 'none',
      ...style
    }
  }, rest), /*#__PURE__*/React.createElement("span", {
    "aria-hidden": true,
    style: {
      fontFamily: 'var(--font-mono)',
      fontSize: sz.glyph,
      lineHeight: 1,
      animation: s.pulse || showBeacon && !solid ? 'cp-live-pulse 1.6s var(--ease-inout) infinite' : 'none'
    }
  }, s.glyph), label || s.label);
}
Object.assign(__ds_scope, { STATUS, StatusPill });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/status/StatusPill.jsx", error: String((e && e.message) || e) }); }

// components/status/UsageMeter.jsx
try { (() => {
// UsageMeter — context / token / cost capacity meter. Bar or ring.
// Fill color escalates by threshold (normal -> warn -> risk -> stop).

function stopColor(pct) {
  if (pct >= 0.92) return 'var(--cap-stop)';
  if (pct >= 0.80) return 'var(--cap-risk)';
  if (pct >= 0.65) return 'var(--cap-warn)';
  return 'var(--cap-normal)';
}
function UsageMeter({
  value = 0,
  max = 100,
  variant = 'bar',
  label,
  valueText,
  accuracy,
  // 'exact' | 'estimated' | 'unavailable'
  size = 'md',
  style = {}
}) {
  const pct = max > 0 ? Math.min(1, Math.max(0, value / max)) : 0;
  const color = accuracy === 'unavailable' ? 'var(--slate)' : stopColor(pct);
  const acc = accuracy && accuracy !== 'exact' ? /*#__PURE__*/React.createElement("span", {
    style: {
      fontSize: 9,
      color: 'var(--text-faint)',
      fontFamily: 'var(--font-mono)'
    }
  }, accuracy === 'estimated' ? '≈' : 'n/a') : null;
  if (variant === 'ring') {
    const d = size === 'sm' ? 26 : 34;
    const sw = size === 'sm' ? 3 : 4;
    const r = (d - sw) / 2;
    const c = 2 * Math.PI * r;
    return /*#__PURE__*/React.createElement("span", {
      title: label,
      style: {
        display: 'inline-flex',
        alignItems: 'center',
        gap: 7,
        ...style
      }
    }, /*#__PURE__*/React.createElement("svg", {
      width: d,
      height: d,
      style: {
        transform: 'rotate(-90deg)',
        flex: 'none'
      }
    }, /*#__PURE__*/React.createElement("circle", {
      cx: d / 2,
      cy: d / 2,
      r: r,
      fill: "none",
      stroke: "var(--cap-track)",
      strokeWidth: sw
    }), /*#__PURE__*/React.createElement("circle", {
      cx: d / 2,
      cy: d / 2,
      r: r,
      fill: "none",
      stroke: color,
      strokeWidth: sw,
      strokeDasharray: c,
      strokeDashoffset: c * (1 - pct),
      strokeLinecap: "round"
    })), (valueText || label) && /*#__PURE__*/React.createElement("span", {
      style: {
        display: 'inline-flex',
        flexDirection: 'column',
        lineHeight: 1.25
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        font: 'var(--fw-medium) var(--fs-meta)/1 var(--font-mono)',
        color
      }
    }, valueText || `${Math.round(pct * 100)}%`), label && /*#__PURE__*/React.createElement("span", {
      style: {
        fontSize: 10,
        color: 'var(--text-faint)'
      }
    }, label)));
  }
  const h = size === 'sm' ? 4 : 6;
  return /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'inline-flex',
      flexDirection: 'column',
      gap: 3,
      minWidth: 96,
      ...style
    }
  }, (label || valueText) && /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 5,
      font: 'var(--fw-regular) var(--fs-meta)/1 var(--font-sans)',
      color: 'var(--text-muted)'
    }
  }, label, (valueText || acc) && /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: 'auto',
      display: 'inline-flex',
      gap: 4,
      alignItems: 'center',
      fontFamily: 'var(--font-mono)',
      color: pct >= 0.8 ? color : 'var(--text-secondary)'
    }
  }, acc, valueText)), /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'block',
      height: h,
      borderRadius: '999px',
      background: 'var(--cap-track)',
      overflow: 'hidden'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'block',
      height: '100%',
      width: `${pct * 100}%`,
      background: color,
      borderRadius: '999px',
      transition: 'width var(--dur-3) var(--ease-out)'
    }
  })));
}
Object.assign(__ds_scope, { UsageMeter });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/status/UsageMeter.jsx", error: String((e && e.message) || e) }); }

// components/objects/SessionRow.jsx
try { (() => {
// SessionRow — the atomic operational unit as a dense, selectable row.
// Composes the status / badge / meter primitives. Used in the sidebar,
// the Sessions list, and the Command Center.

const ATTN = {
  'waiting-human': 5,
  approval: 5,
  failed: 4,
  blocked: 4,
  conflict: 4,
  degraded: 3,
  stale: 3,
  running: 2,
  editing: 2,
  testing: 2,
  active: 1,
  'pr-open': 1,
  idle: 0,
  completed: 0,
  archived: 0
};
function SessionRow({
  title = 'Untitled session',
  status = 'idle',
  harness = 'claude-code',
  profile = 'Claude Max Main',
  provider = 'claude',
  task,
  // { id, tone } e.g. { id:'ENG-221', tone:'linear' }
  branch,
  // string
  worktree,
  // string
  pr,
  // string e.g. '#84'
  context,
  // { value, max, text }
  activity = '',
  // last activity text
  current = '',
  // current command / activity line
  selected = false,
  density = 'comfortable',
  // 'comfortable' | 'compact'
  onClick,
  style = {}
}) {
  const level = ATTN[status] ?? 0;
  const compact = density === 'compact';
  return /*#__PURE__*/React.createElement("div", {
    onClick: onClick,
    role: "row",
    "aria-selected": selected,
    style: {
      display: 'flex',
      alignItems: 'stretch',
      gap: 0,
      background: selected ? 'var(--surface-active)' : 'transparent',
      borderRadius: 'var(--r-2)',
      cursor: 'pointer',
      overflow: 'hidden',
      boxShadow: selected ? 'inset 0 0 0 1px var(--accent-line)' : 'inset 0 0 0 1px transparent',
      transition: 'background var(--dur-1) var(--ease-standard)',
      ...style
    },
    onMouseEnter: e => {
      if (!selected) e.currentTarget.style.background = 'var(--surface-hover)';
    },
    onMouseLeave: e => {
      if (!selected) e.currentTarget.style.background = 'transparent';
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.AttentionMarker, {
    level: level
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      minWidth: 0,
      padding: compact ? '6px 10px' : '8px 11px',
      display: 'flex',
      flexDirection: 'column',
      gap: compact ? 4 : 6
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      minWidth: 0
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.StatusPill, {
    status: status,
    size: "xs"
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      font: `var(--fw-medium) var(--fs-body)/1.2 var(--font-sans)`,
      color: 'var(--text-primary)',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap',
      minWidth: 0,
      flex: 1
    }
  }, title), context && /*#__PURE__*/React.createElement(__ds_scope.UsageMeter, {
    variant: "ring",
    size: "sm",
    value: context.value,
    max: context.max,
    label: "ctx"
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 6,
      flexWrap: 'wrap'
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.HarnessBadge, {
    harness: harness
  }), /*#__PURE__*/React.createElement(__ds_scope.ProfileBadge, {
    name: profile,
    provider: provider
  }), task && /*#__PURE__*/React.createElement(__ds_scope.MetaChip, {
    tone: task.tone || 'linear',
    mono: false
  }, task.id), branch && /*#__PURE__*/React.createElement(__ds_scope.MetaChip, {
    tone: "branch"
  }, branch), worktree && /*#__PURE__*/React.createElement(__ds_scope.MetaChip, {
    tone: "worktree"
  }, worktree), pr && /*#__PURE__*/React.createElement(__ds_scope.MetaChip, {
    tone: "pr"
  }, pr)), (current || activity) && !compact && /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      font: 'var(--fw-regular) var(--fs-meta)/1.3 var(--font-mono)',
      color: 'var(--text-muted)',
      minWidth: 0
    }
  }, current && /*#__PURE__*/React.createElement("span", {
    style: {
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap',
      minWidth: 0
    }
  }, current), activity && /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: 'auto',
      flex: 'none',
      color: 'var(--text-faint)'
    }
  }, activity))));
}
Object.assign(__ds_scope, { SessionRow });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/objects/SessionRow.jsx", error: String((e && e.message) || e) }); }

// ui_kits/control-plane/kit-data.js
try { (() => {
/* Sample data for the Control Plane UI kit. Realistic, per the spec. */
window.KIT = {
  projects: [{
    id: 'cp',
    name: 'AI Engineering Control Plane',
    repo: 'org/control-plane',
    active: 3,
    waiting: 1,
    prs: 2,
    workflow: 'active',
    brain: 'ready'
  }, {
    id: 'pb',
    name: 'Project Brain',
    repo: 'org/project-brain',
    active: 1,
    waiting: 0,
    prs: 1,
    workflow: 'drift',
    brain: 'indexing'
  }, {
    id: 'cc',
    name: 'cc-crew Scaffold Demo',
    repo: 'org/cc-crew-demo',
    active: 0,
    waiting: 0,
    prs: 0,
    workflow: 'needs-personalization',
    brain: 'stale'
  }, {
    id: 'rg',
    name: 'RepoGraph Parser',
    repo: 'org/repograph',
    active: 1,
    waiting: 1,
    prs: 0,
    workflow: 'none',
    brain: 'ready'
  }, {
    id: 'wc',
    name: 'Weekly Commit Automation',
    repo: 'org/weekly-commit',
    active: 0,
    waiting: 0,
    prs: 1,
    workflow: 'active',
    brain: 'ready'
  }],
  sessions: [{
    id: 's1',
    proj: 'cp',
    status: 'waiting-human',
    title: 'ENG-221 · GitHub OAuth callback',
    harness: 'claude-code',
    provider: 'claude',
    profile: 'Claude Max Main',
    task: {
      id: 'ENG-221',
      tone: 'linear'
    },
    branch: 'agent/eng-221-oauth',
    worktree: '~/wt/eng-221',
    pr: '#84',
    context: {
      value: 186,
      max: 200
    },
    current: '$ npm test — awaiting permission',
    activity: '2m ago'
  }, {
    id: 's2',
    proj: 'cp',
    status: 'running',
    title: 'GH-184 · parser memory leak',
    harness: 'codex-cli',
    provider: 'codex',
    profile: 'Codex CLI Main',
    task: {
      id: '#184',
      tone: 'github'
    },
    branch: 'fix/gh-184-leak',
    worktree: '~/wt/gh-184',
    context: {
      value: 96,
      max: 200
    },
    current: 'editing src/parser/stream.ts',
    activity: 'just now'
  }, {
    id: 's3',
    proj: 'cp',
    status: 'active',
    title: 'Phase 2 · Observability Graph',
    team: true,
    harness: 'claude-code',
    provider: 'claude',
    profile: 'Claude Team Work',
    task: {
      id: 'P2.3',
      tone: 'accent'
    },
    branch: 'team/phase-2-graph',
    worktree: '~/wt/phase-2',
    context: {
      value: 132,
      max: 200
    },
    current: 'orchestrator delegating 3 workers',
    activity: '40s ago'
  }, {
    id: 's4',
    proj: 'rg',
    status: 'failed',
    title: 'PR checks fix — snapshot drift',
    harness: 'codex-cli',
    provider: 'codex',
    profile: 'Codex Cloud GitHub',
    task: {
      id: '#71',
      tone: 'github'
    },
    branch: 'fix/snapshots',
    worktree: '~/wt/snap',
    context: {
      value: 58,
      max: 200
    },
    current: 'check "unit" failed — exit 1',
    activity: '6m ago'
  }, {
    id: 's5',
    proj: 'cp',
    status: 'completed',
    title: 'Docs drift refresh',
    harness: 'claude-code',
    provider: 'claude',
    profile: 'Claude Team Work',
    branch: 'chore/docs-drift',
    pr: '#82',
    context: {
      value: 40,
      max: 200
    },
    current: 'summarized to Project Brain',
    activity: '14m ago'
  }],
  profiles: [{
    name: 'Claude Max Main',
    provider: 'claude',
    health: 'active'
  }, {
    name: 'Claude Max Secondary',
    provider: 'claude',
    health: 'available'
  }, {
    name: 'Claude Team Work',
    provider: 'claude',
    health: 'rate-limited'
  }, {
    name: 'Codex CLI Main',
    provider: 'codex',
    health: 'active'
  }, {
    name: 'Codex Cloud GitHub',
    provider: 'codex',
    health: 'auth-expired'
  }],
  prs: [{
    id: '#84',
    proj: 'cp',
    title: 'Add workflow command registry',
    branch: 'agent/eng-221-oauth',
    base: 'main',
    lane: 'open',
    status: 'pr-open',
    checks: 'failing',
    author: 'Codex · PR fix',
    adds: 412,
    dels: 98,
    files: 6,
    comments: 2,
    age: '2h'
  }, {
    id: '#82',
    proj: 'cp',
    title: 'Docs drift refresh',
    branch: 'chore/docs-drift',
    base: 'main',
    lane: 'ready',
    status: 'pr-ready',
    checks: 'passing',
    author: 'Claude · Docs drift',
    adds: 64,
    dels: 12,
    files: 3,
    comments: 0,
    age: '14m'
  }, {
    id: '#80',
    proj: 'cp',
    title: 'Risk taxonomy + gateway scaffolding',
    branch: 'feat/risk-enum',
    base: 'main',
    lane: 'merged',
    status: 'pr-open',
    checks: 'passing',
    author: 'Claude · ENG-210',
    adds: 220,
    dels: 40,
    files: 9,
    comments: 5,
    age: '1d'
  }, {
    id: '#71',
    proj: 'rg',
    title: 'Snapshot regeneration',
    branch: 'fix/snapshots',
    base: 'main',
    lane: 'open',
    status: 'pr-open',
    checks: 'failing',
    author: 'Codex · #71',
    adds: 12,
    dels: 4,
    files: 1,
    comments: 1,
    age: '6m'
  }],
  worktrees: [{
    id: 'wt1',
    proj: 'cp',
    path: '~/wt/eng-221',
    branch: 'agent/eng-221-oauth',
    base: 'main',
    status: 'dirty',
    dirty: 7,
    session: 'ENG-221 · OAuth',
    task: {
      id: 'ENG-221',
      tone: 'linear'
    },
    commit: '4f18a70',
    pr: '#84',
    checks: 'failing',
    risk: 'medium'
  }, {
    id: 'wt2',
    proj: 'cp',
    path: '~/wt/gh-184',
    branch: 'fix/gh-184-leak',
    base: 'main',
    status: 'dirty',
    dirty: 3,
    session: 'GH-184 · leak',
    task: {
      id: '#184',
      tone: 'github'
    },
    commit: 'a91c2d1',
    pr: null,
    checks: null,
    risk: 'low'
  }, {
    id: 'wt3',
    proj: 'cp',
    path: '~/wt/phase-2',
    branch: 'team/phase-2-graph',
    base: 'main',
    status: 'active',
    dirty: 12,
    session: 'Phase 2 team',
    task: {
      id: 'P2.3',
      tone: 'accent'
    },
    commit: '7b3f1e0',
    pr: null,
    checks: null,
    risk: 'low'
  }, {
    id: 'wt4',
    proj: 'rg',
    path: '~/wt/snap',
    branch: 'fix/snapshots',
    base: 'main',
    status: 'conflict',
    dirty: 4,
    session: '#71 · snapshot fix',
    task: {
      id: '#71',
      tone: 'github'
    },
    commit: 'e22a9c4',
    pr: '#71',
    checks: 'failing',
    risk: 'high'
  }],
  // ---- Usage / cost ----
  usage: {
    today: {
      tokensIn: 1284000,
      tokensOut: 412000,
      spend: 34,
      spendLimit: 50,
      sessions: 5,
      context: 512,
      contextMax: 1000
    },
    // last 14 days of spend ($)
    spend14: [12, 18, 9, 22, 31, 14, 6, 19, 27, 24, 38, 41, 29, 34],
    byProfile: [{
      name: 'Claude Max Main',
      provider: 'claude',
      spend: 14.2,
      tokens: 720000,
      pct: 42
    }, {
      name: 'Claude Team Work',
      provider: 'claude',
      spend: 9.6,
      tokens: 480000,
      pct: 28
    }, {
      name: 'Codex CLI Main',
      provider: 'codex',
      spend: 6.8,
      tokens: 360000,
      pct: 20
    }, {
      name: 'Claude Max Secondary',
      provider: 'claude',
      spend: 3.4,
      tokens: 136000,
      pct: 10
    }],
    topContext: [{
      session: 'ENG-221 · OAuth',
      value: 186,
      max: 200
    }, {
      session: 'Phase 2 · Graph',
      value: 132,
      max: 200
    }, {
      session: 'GH-184 · leak',
      value: 96,
      max: 200
    }]
  },
  events: [{
    t: '14:22:08',
    kind: 'approval',
    actor: 'Claude ENG-221',
    text: 'requested permission to run npm test',
    risk: 'medium'
  }, {
    t: '14:21:54',
    kind: 'git',
    actor: 'Codex GH-184',
    text: 'committed 3 files to fix/gh-184-leak'
  }, {
    t: '14:20:11',
    kind: 'brain',
    actor: 'Project Brain',
    text: 'proposed action plan · 4 steps · create worktree + /team-start'
  }, {
    t: '14:18:02',
    kind: 'pr',
    actor: 'Codex PR-fix',
    text: 'opened PR #84 — checks running'
  }, {
    t: '14:15:40',
    kind: 'workflow',
    actor: 'cc-crew',
    text: '/team-start backend launched 3 workers'
  }, {
    t: '14:12:19',
    kind: 'session',
    actor: 'Claude Docs',
    text: 'session completed · summarized to Project Brain'
  }],
  diff: [{
    file: 'src/gateway/review.ts',
    header: '@@ -10,4 +10,6 @@',
    comments: 2,
    lines: [{
      type: 'ctx',
      ln: 10,
      text: 'export async function review(plan: ActionPlan) {'
    }, {
      type: 'del',
      text: '  return execute(plan)'
    }, {
      type: 'add',
      ln: 11,
      text: '  const dry = await dryRun(plan)'
    }, {
      type: 'add',
      ln: 12,
      text: '  if (dry.risk === "critical") return gateway.requireTyped(dry)'
    }, {
      type: 'add',
      ln: 13,
      text: '  return gateway.confirm(dry)'
    }, {
      type: 'ctx',
      ln: 14,
      text: '}'
    }]
  }, {
    file: 'src/gateway/risk.ts',
    header: '@@ -3,2 +3,5 @@',
    comments: 0,
    lines: [{
      type: 'ctx',
      ln: 3,
      text: 'export type Risk ='
    }, {
      type: 'add',
      ln: 4,
      text: "  | 'readonly' | 'low' | 'medium'"
    }, {
      type: 'add',
      ln: 5,
      text: "  | 'high' | 'critical'"
    }]
  }],
  // ---- Editor / IDE ----
  tree: [{
    type: 'dir',
    name: 'src',
    open: true,
    depth: 0
  }, {
    type: 'dir',
    name: 'gateway',
    open: true,
    depth: 1,
    git: 'M'
  }, {
    type: 'file',
    name: 'review.ts',
    depth: 2,
    git: 'M',
    active: true,
    agent: true
  }, {
    type: 'file',
    name: 'risk.ts',
    depth: 2,
    git: 'M'
  }, {
    type: 'file',
    name: 'confirm.ts',
    depth: 2
  }, {
    type: 'dir',
    name: 'parser',
    open: false,
    depth: 1
  }, {
    type: 'dir',
    name: 'brain',
    open: false,
    depth: 1
  }, {
    type: 'file',
    name: 'index.ts',
    depth: 1
  }, {
    type: 'dir',
    name: 'tests',
    open: false,
    depth: 0,
    git: 'M'
  }, {
    type: 'file',
    name: 'package.json',
    depth: 0
  }],
  tabs: [{
    name: 'review.ts',
    path: 'src/gateway',
    dirty: true,
    active: true,
    agent: true
  }, {
    name: 'risk.ts',
    path: 'src/gateway',
    dirty: true
  }, {
    name: 'gateway.test.ts',
    path: 'tests'
  }],
  // each line: { n, t (text), c (class: kw/str/com/type/fn/num/punct mix handled in-component), mark }
  code: ["import { ActionPlan } from './plan'", "import { dryRun } from './sandbox'", "import { gateway } from './confirm'", "", "// Risk-gated execution — every action passes the Gateway", "export async function review(plan: ActionPlan) {", "  const dry = await dryRun(plan)", "  if (dry.risk === 'critical') {", "    return gateway.requireTyped(dry)", "  }", "  return gateway.confirm(dry)", "}", "", "export function classify(plan: ActionPlan): Risk {", "  return plan.writes ? 'high' : 'readonly'", "}"],
  // gutter marks by line index (0-based)
  codeMarks: {
    6: 'add',
    7: 'add',
    8: 'add',
    9: 'add',
    14: 'ctx'
  },
  agentEdit: {
    line: 8,
    who: 'Claude · ENG-221',
    note: 'inserting critical-risk gate'
  },
  problems: [{
    sev: 'warn',
    text: "'Risk' type imported but used before its declaration",
    at: 'review.ts:14'
  }],
  // per-file content for the editor (keyed by filename)
  files: {
    'review.ts': {
      path: 'src/gateway',
      dirty: true,
      agent: true,
      lines: ["import { ActionPlan } from './plan'", "import { dryRun } from './sandbox'", "import { gateway } from './confirm'", "", "// Risk-gated execution — every action passes the Gateway", "export async function review(plan: ActionPlan) {", "  const dry = await dryRun(plan)", "  if (dry.risk === 'critical') {", "    return gateway.requireTyped(dry)", "  }", "  return gateway.confirm(dry)", "}", "", "export function classify(plan: ActionPlan): Risk {", "  return plan.writes ? 'high' : 'readonly'", "}"],
      marks: {
        6: 'add',
        7: 'add',
        8: 'add',
        9: 'add'
      },
      agentEdit: {
        line: 8,
        who: 'Claude · ENG-221',
        note: 'inserting critical-risk gate'
      },
      problems: [{
        sev: 'warn',
        text: "'Risk' type imported but used before its declaration",
        at: 'review.ts:14'
      }]
    },
    'risk.ts': {
      path: 'src/gateway',
      dirty: true,
      agent: false,
      lines: ["// Risk taxonomy for the Action Gateway", "export type Risk =", "  | 'readonly'", "  | 'low'", "  | 'medium'", "  | 'high'", "  | 'critical'", "", "export const ORDER: Risk[] = [", "  'readonly', 'low', 'medium', 'high', 'critical'", "]"],
      marks: {
        5: 'add',
        6: 'add'
      },
      agentEdit: null,
      problems: []
    },
    'gateway.test.ts': {
      path: 'tests',
      dirty: false,
      agent: false,
      lines: ["import { review } from '../src/gateway/review'", "import { classify } from '../src/gateway/risk'", "", "test('critical actions require typed confirm', async () => {", "  const plan = { writes: true, risk: 'critical' }", "  expect(classify(plan)).toBe('high')", "  // snapshot below is stale — regenerate with -u", "  expect(await review(plan)).toMatchSnapshot()", "})"],
      marks: {
        6: 'del'
      },
      agentEdit: null,
      problems: [{
        sev: 'fail',
        text: 'snapshot mismatch — Risk enum changed',
        at: 'gateway.test.ts:8'
      }]
    },
    'confirm.ts': {
      path: 'src/gateway',
      dirty: false,
      agent: false,
      lines: ["import { Risk } from './risk'", "", "export const gateway = {", "  confirm: (dry: DryRun) => emit('approval.request', dry),", "  requireTyped: (dry: DryRun) => emit('approval.typed', dry),", "}"],
      marks: {},
      agentEdit: null,
      problems: []
    }
  },
  // ---- Agent Team ----
  team: {
    name: 'Phase 2 · Observability Graph',
    pack: 'cc-crew',
    lead: {
      role: 'Orchestrator',
      harness: 'claude-code',
      profile: 'Claude Team Work',
      status: 'active',
      task: 'decomposing into 3 workstreams',
      ctx: 132
    },
    workers: [{
      id: 'w1',
      role: 'Graph renderer',
      harness: 'claude-code',
      status: 'running',
      task: 'editing GraphCanvas.tsx',
      ctx: 88,
      wt: '~/wt/phase-2/w1'
    }, {
      id: 'w2',
      role: 'Status adapters',
      harness: 'codex-cli',
      status: 'waiting-perm',
      task: 'wants to install d3-force',
      ctx: 41,
      wt: '~/wt/phase-2/w2'
    }, {
      id: 'w3',
      role: 'Layout solver',
      harness: 'claude-code',
      status: 'completed',
      task: 'opened PR #86',
      ctx: 70,
      wt: '~/wt/phase-2/w3'
    }]
  },
  // ---- Action Gateway queue items ----
  gateway: {
    q1: {
      id: 'q1',
      risk: 'medium',
      who: 'Claude · ENG-221 · Claude Max Main',
      short: 'Permission to run npm test',
      title: 'Run npm test',
      cmd: 'npm test',
      wt: '~/wt/eng-221',
      desc: 'Sandboxed in worktree. No network. Reversible.',
      conseq: [['terminal', 'Execute test runner (jest) in worktree'], ['folder', 'Read access to src/** and test fixtures'], ['shield-off', 'No writes outside worktree · no push']]
    },
    q2: {
      id: 'q2',
      risk: 'high',
      who: 'Codex · GH-184 · Codex Cloud',
      short: 'Approve force-push to fix/gh-184-leak',
      title: 'Force-push fix/gh-184-leak',
      cmd: 'git push --force-with-lease',
      wt: '~/wt/gh-184',
      desc: 'Rewrites remote history on a shared branch.',
      conseq: [['git-branch', 'Overwrite remote fix/gh-184-leak'], ['users-round', 'Affects anyone tracking the branch'], ['rotate-ccw', 'Reversible only via reflog']]
    },
    edit: {
      id: 'edit',
      risk: 'medium',
      who: 'Claude · ENG-221 · Claude Max Main',
      short: 'Apply agent edit to review.ts',
      title: 'Apply 4-line edit to review.ts',
      cmd: 'write src/gateway/review.ts',
      wt: '~/wt/eng-221',
      desc: 'Inserts a critical-risk gate, then runs the suite.',
      conseq: [['file-code', 'Modify src/gateway/review.ts (+4 −1)'], ['terminal', 'Run jest after applying'], ['shield-off', 'No push']]
    },
    perm: {
      id: 'q1',
      risk: 'medium',
      who: 'Claude · ENG-221 · Claude Max Main',
      short: 'Permission to run npm test',
      title: 'Run npm test',
      cmd: 'npm test',
      wt: '~/wt/eng-221',
      desc: 'Sandboxed in worktree. No network. Reversible.',
      conseq: [['terminal', 'Execute test runner (jest) in worktree'], ['folder', 'Read access to src/** and test fixtures'], ['shield-off', 'No writes outside worktree · no push']]
    }
  },
  // ---- Project Brain (co-pilot chat) ----
  brainThread: [{
    from: 'user',
    text: 'Why are PR #84 checks failing?'
  }, {
    from: 'brain',
    text: "The **unit** check fails because the new `Risk` enum added `'critical'`, but the snapshot in `review.test.ts` wasn't regenerated. Nothing else in the suite is affected.",
    evidence: [{
      kind: 'commit',
      label: '4f18a70',
      sub: 'add critical risk gate'
    }, {
      kind: 'pr',
      label: '#84',
      sub: 'checks failing'
    }, {
      kind: 'anchor',
      label: 'review.test.ts#snapshot',
      freshness: 'stale'
    }],
    plan: {
      title: 'Fix snapshot drift',
      steps: [{
        risk: 'readonly',
        text: 'Open review.test.ts at failing snapshot'
      }, {
        risk: 'low',
        text: 'Regenerate snapshot — jest -u'
      }, {
        risk: 'medium',
        text: 'Commit to fix/snapshots and re-run checks'
      }]
    }
  }],
  // canned co-pilot replies keyed by loose intent
  brainReplies: [{
    match: /next|backend|start|task/i,
    text: "Next up is **PlanTask `phase-2-backend-auth`**. I can spin it up: new worktree, a Claude Code session on Claude Max Main, linked to the plan task, with a generated prompt.",
    evidence: [{
      kind: 'plantask',
      label: 'phase-2-backend-auth',
      sub: 'ready'
    }, {
      kind: 'anchor',
      label: 'ARCHITECTURE.md#auth'
    }, {
      kind: 'memory',
      label: 'decision: OAuth via gateway'
    }],
    plan: {
      title: 'Start backend auth task',
      steps: [{
        risk: 'low',
        text: 'Create worktree agent/p2-backend-auth'
      }, {
        risk: 'low',
        text: 'Start Claude Code session · Claude Max Main'
      }, {
        risk: 'readonly',
        text: 'Link session to PlanTask phase-2-backend-auth'
      }, {
        risk: 'medium',
        text: 'Send generated task prompt + open terminal'
      }]
    }
  }, {
    match: /test|fail|check|fix/i,
    text: "The failing check is snapshot drift in `review.test.ts`. Regenerating the snapshot and re-running checks should clear it. Want me to stage that as a plan?",
    evidence: [{
      kind: 'pr',
      label: '#84',
      sub: 'checks failing'
    }, {
      kind: 'commit',
      label: '4f18a70',
      sub: 'add critical risk gate'
    }]
  }, {
    match: /.*/,
    text: "Here's what I can ground that in across the project — code, docs, commits, sessions, PRs and plan tasks. Ask me to *Plan* an action and I'll draft it for the Action Gateway.",
    evidence: [{
      kind: 'memory',
      label: '142 indexed objects'
    }, {
      kind: 'decision',
      label: 'gateway-first execution'
    }]
  }],
  brainMemory: [{
    kind: 'decision',
    label: 'Gateway-first execution',
    sub: 'all actions risk-rated',
    t: '2d ago'
  }, {
    kind: 'decision',
    label: 'OAuth via callback gate',
    sub: 'ENG-221',
    t: '4h ago'
  }, {
    kind: 'memory',
    label: 'Worktree-per-session',
    sub: 'isolation invariant',
    t: '1w ago'
  }, {
    kind: 'anchor',
    label: 'ARCHITECTURE.md#gateway',
    sub: 'grounded @ 4f18a70',
    t: 'fresh'
  }],
  // ---- Audit / Event timeline ----
  audit: [{
    t: '14:22:08',
    kind: 'approval',
    proj: 'cp',
    actor: 'You',
    target: 'Claude · ENG-221',
    text: 'approved — run npm test',
    risk: 'medium',
    result: 'approved'
  }, {
    t: '14:22:01',
    kind: 'approval',
    proj: 'cp',
    actor: 'Claude · ENG-221',
    text: 'requested permission to run npm test',
    risk: 'medium',
    result: 'pending'
  }, {
    t: '14:21:54',
    kind: 'git',
    proj: 'cp',
    actor: 'Codex · GH-184',
    text: 'committed 3 files to fix/gh-184-leak',
    meta: '4f18a70'
  }, {
    t: '14:21:30',
    kind: 'session',
    proj: 'cp',
    actor: 'Codex · GH-184',
    text: 'session resumed after rate-limit backoff'
  }, {
    t: '14:20:11',
    kind: 'brain',
    proj: 'cp',
    actor: 'Project Brain',
    text: 'proposed action plan · 4 steps · create worktree + /team-start'
  }, {
    t: '14:19:40',
    kind: 'session',
    proj: 'rg',
    actor: 'Codex · #71',
    text: 'check "unit" failed — snapshot drift',
    result: 'pending'
  }, {
    t: '14:19:02',
    kind: 'gateway',
    proj: 'cp',
    actor: 'You',
    target: 'Codex · GH-184',
    text: 'denied — force-push to main',
    risk: 'critical',
    result: 'denied'
  }, {
    t: '14:18:02',
    kind: 'pr',
    proj: 'cp',
    actor: 'Codex · PR-fix',
    text: 'opened PR #84 — checks running',
    meta: '#84'
  }, {
    t: '14:16:30',
    kind: 'git',
    proj: 'rg',
    actor: 'Codex · #71',
    text: 'created worktree ~/wt/snap',
    meta: 'fix/snapshots'
  }, {
    t: '14:15:40',
    kind: 'workflow',
    proj: 'cp',
    actor: 'cc-crew',
    text: '/team-start backend launched 3 workers'
  }, {
    t: '14:14:21',
    kind: 'profile',
    proj: 'cp',
    actor: 'Runtime',
    text: 'Claude Team Work entered rate-limited state'
  }, {
    t: '14:12:19',
    kind: 'session',
    proj: 'cp',
    actor: 'Claude · Docs',
    text: 'session completed · summarized to Project Brain'
  }, {
    t: '14:09:50',
    kind: 'git',
    proj: 'cp',
    actor: 'Claude · ENG-221',
    text: 'created worktree ~/wt/eng-221',
    meta: 'agent/eng-221-oauth'
  }],
  auditFilters: ['All', 'Approvals', 'Git', 'Sessions', 'Brain', 'Workflow'],
  // ---- Settings / Execution profiles ----
  profilesDetail: [{
    name: 'Claude Max Main',
    provider: 'claude',
    health: 'active',
    harness: 'claude-code',
    sessions: 2,
    usage: 62,
    limit: 100,
    resets: '—',
    note: 'Primary interactive profile'
  }, {
    name: 'Claude Max Secondary',
    provider: 'claude',
    health: 'available',
    harness: 'claude-code',
    sessions: 0,
    usage: 8,
    limit: 100,
    resets: '—',
    note: 'Overflow / parallel work'
  }, {
    name: 'Claude Team Work',
    provider: 'claude',
    health: 'rate-limited',
    harness: 'claude-code',
    sessions: 1,
    usage: 98,
    limit: 100,
    resets: 'in 24m',
    note: 'Shared team seat'
  }, {
    name: 'Codex CLI Main',
    provider: 'codex',
    health: 'active',
    harness: 'codex-cli',
    sessions: 1,
    usage: 34,
    limit: 80,
    resets: '—',
    note: 'Local CLI harness'
  }, {
    name: 'Codex Cloud GitHub',
    provider: 'codex',
    health: 'auth-expired',
    harness: 'codex-cloud',
    sessions: 0,
    usage: 0,
    limit: 80,
    resets: '—',
    note: 'Re-auth required'
  }],
  // ---- Workflow Packs (pack ≠ instance) ----
  packs: [{
    id: 'cc-crew',
    name: 'cc-crew',
    provider: 'bundled',
    version: '1.4.0',
    instance: 'active',
    project: 'Control Plane',
    desc: 'Multi-agent backend crew — orchestrator delegates to implementer + reviewer workers.',
    commands: [{
      name: '/team-start',
      type: 'recipe',
      needsInstance: true,
      creates: 'agent team'
    }, {
      name: '/plan-sync',
      type: 'slash',
      needsInstance: true
    }, {
      name: '/review-pass',
      type: 'slash',
      needsInstance: false
    }],
    roles: ['Orchestrator', 'Implementer', 'Reviewer'],
    parser: 'PHASE_PLAN.md',
    recipes: 2,
    drift: null
  }, {
    id: 'docs-refresh',
    name: 'docs-refresh',
    provider: 'user',
    version: '0.9.0',
    instance: 'ready',
    project: 'Control Plane',
    desc: 'Keeps architecture docs synced to code; runs on a schedule or on drift.',
    commands: [{
      name: '/docs-drift',
      type: 'slash',
      needsInstance: false
    }],
    roles: [],
    parser: null,
    recipes: 1,
    drift: null
  }, {
    id: 'weekly-commit',
    name: 'weekly-commit',
    provider: 'third_party',
    version: '2.1.0',
    instance: 'upgrade_available',
    project: 'Weekly Commit Automation',
    desc: 'Scheduled commit + summary automation. A newer pack version is available.',
    commands: [{
      name: '/weekly',
      type: 'slash',
      needsInstance: false
    }],
    roles: [],
    parser: null,
    recipes: 1,
    drift: 'upgrade'
  }, {
    id: 'rg-scaffold',
    name: 'repograph-scaffold',
    provider: 'user',
    version: '0.3.0',
    instance: 'needs_personalization',
    project: 'RepoGraph Parser',
    desc: 'Template scaffold. Must be personalized against this project’s architecture before its commands can run.',
    commands: [{
      name: '/rg-init',
      type: 'recipe',
      needsInstance: true,
      creates: 'session'
    }],
    roles: ['Mapper'],
    parser: 'CODE_AREAS.yaml',
    recipes: 1,
    drift: null
  }],
  // ---- Task intake (GitHub issues · Linear tickets · plan tasks) ----
  tasks: [{
    source: 'linear',
    id: 'ENG-221',
    title: 'Add GitHub OAuth callback',
    priority: 'High',
    labels: ['auth', 'backend'],
    status: 'In progress',
    planTask: 'Phase 2.1',
    harness: 'claude-code',
    profile: 'Claude Max Main',
    branch: 'agent/eng-221-oauth',
    session: 's1'
  }, {
    source: 'github',
    id: '#184',
    title: 'Fix parser memory leak',
    priority: 'Urgent',
    labels: ['bug', 'parser'],
    status: 'In progress',
    planTask: null,
    harness: 'codex-cli',
    profile: 'Codex CLI Main',
    branch: 'fix/gh-184-leak',
    session: 's2'
  }, {
    source: 'plan',
    id: 'Phase 2.3',
    title: 'Project observability graph',
    priority: 'High',
    labels: ['phase-2'],
    status: 'Ready',
    planTask: null,
    harness: 'claude-code',
    profile: 'Claude Team Work'
  }, {
    source: 'plan',
    id: 'Phase 3.1',
    title: 'Action Gateway approval cards',
    priority: 'Medium',
    labels: ['phase-3', 'gateway'],
    status: 'Backlog',
    planTask: null,
    harness: 'claude-code',
    profile: 'Claude Max Main'
  }, {
    source: 'linear',
    id: 'ENG-240',
    title: 'Token usage meter accuracy',
    priority: 'Medium',
    labels: ['usage'],
    status: 'Todo',
    planTask: 'Phase 3.4',
    harness: 'claude-code',
    profile: 'Claude Max Secondary'
  }, {
    source: 'github',
    id: '#190',
    title: 'Workflow pack registry schema',
    priority: 'Low',
    labels: ['workflow'],
    status: 'Todo',
    planTask: null,
    harness: 'codex-cli',
    profile: 'Codex CLI Main'
  }, {
    source: 'linear',
    id: 'ENG-205',
    title: 'Audit trail export to NDJSON',
    priority: 'Low',
    labels: ['audit'],
    status: 'Done',
    planTask: null,
    harness: 'claude-code',
    profile: 'Claude Max Main'
  }],
  // ---- Implementation plan (Phase → Track → PlanTask) ----
  plan: {
    name: 'Control Plane — Implementation Plan',
    source: 'PHASE_PLAN.md',
    parser: 'cc-crew',
    phases: [{
      id: 'p1',
      name: 'Phase 1 · Foundations',
      status: 'completed',
      tracks: [{
        id: 't1a',
        name: 'Track A · Runtime & profiles',
        tasks: [{
          id: '1.1',
          title: 'Local runtime detection',
          status: 'completed',
          ac: 4,
          files: 8,
          anchors: ['ARCHITECTURE.md#runtime'],
          task: null
        }, {
          id: '1.2',
          title: 'Execution profile manager',
          status: 'completed',
          ac: 5,
          files: 11,
          anchors: ['ARCHITECTURE.md#profiles'],
          task: null
        }]
      }]
    }, {
      id: 'p2',
      name: 'Phase 2 · Observability Graph',
      status: 'active',
      tracks: [{
        id: 't2a',
        name: 'Track A · Sessions & graph',
        tasks: [{
          id: '2.1',
          title: 'GitHub OAuth callback',
          status: 'in-progress',
          ac: 3,
          files: 4,
          anchors: ['ARCHITECTURE.md#auth'],
          task: {
            id: 'ENG-221',
            tone: 'linear'
          },
          session: 's1'
        }, {
          id: '2.3',
          title: 'Project observability graph',
          status: 'ready',
          ac: 4,
          files: 0,
          anchors: ['ARCHITECTURE.md#graph'],
          task: {
            id: 'Phase 2.3',
            tone: 'accent'
          }
        }]
      }, {
        id: 't2b',
        name: 'Track B · Status adapters',
        tasks: [{
          id: '2.4',
          title: 'Harness status adapters',
          status: 'in-progress',
          ac: 6,
          files: 7,
          anchors: ['ARCHITECTURE.md#adapters'],
          task: null,
          team: true
        }]
      }]
    }, {
      id: 'p3',
      name: 'Phase 3 · Action Gateway',
      status: 'backlog',
      tracks: [{
        id: 't3a',
        name: 'Track A · Approvals',
        tasks: [{
          id: '3.1',
          title: 'Action Gateway approval cards',
          status: 'backlog',
          ac: 5,
          files: 0,
          anchors: ['ARCHITECTURE.md#gateway'],
          task: {
            id: 'Phase 3.1',
            tone: 'accent'
          }
        }, {
          id: '3.4',
          title: 'Token usage meter accuracy',
          status: 'todo',
          ac: 2,
          files: 0,
          anchors: [],
          task: {
            id: 'ENG-240',
            tone: 'linear'
          }
        }]
      }]
    }]
  },
  // ---- Human Input Queue (centralized triage) ----
  humanInput: [{
    id: 'q1',
    group: 'Permission requests',
    kind: 'gateway',
    risk: 'medium',
    actor: 'Claude · ENG-221',
    target: '~/wt/eng-221',
    reason: 'Run npm test in the worktree sandbox',
    age: '2m'
  }, {
    id: 'q2',
    group: 'High-risk actions',
    kind: 'gateway',
    risk: 'high',
    actor: 'Codex · GH-184',
    target: 'fix/gh-184-leak',
    reason: 'Force-push rewrites shared remote history',
    age: '5m'
  }, {
    id: 'dec-71',
    group: 'Failed checks · needs decision',
    kind: 'decision',
    risk: 'medium',
    actor: 'Codex · #71',
    target: 'RepoGraph Parser',
    reason: 'unit check failed — snapshot drift. Retry, regenerate, or skip?',
    age: '6m'
  }, {
    id: 'pers-rg',
    group: 'Workflow personalization',
    kind: 'personalization',
    risk: 'low',
    actor: 'repograph-scaffold',
    target: 'RepoGraph Parser',
    reason: 'Template pack must be personalized before its commands can run',
    age: '1h'
  }],
  // ---- Integrations (Settings) ----
  integrations: [{
    id: 'runtime',
    name: 'Local runtime',
    icon: 'cpu',
    connected: true,
    detail: 'Healthy · 3 sessions · worktree root ~/wt',
    action: 'Manage'
  }, {
    id: 'github',
    name: 'GitHub',
    icon: 'github',
    connected: true,
    detail: 'org · 12 repos · mapped org/control-plane',
    scope: 'Issues · PRs · checks',
    action: 'Manage'
  }, {
    id: 'linear',
    name: 'Linear',
    icon: 'square-kanban',
    connected: false,
    detail: 'Not connected — link a team to pull tickets into the Task Inbox',
    scope: null,
    action: 'Connect'
  }, {
    id: 'brain',
    name: 'Project Brain store',
    icon: 'brain',
    connected: true,
    detail: 'Ready · 142 objects indexed · grounded @ 4f18a70',
    action: 'Manage'
  }]
};
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/control-plane/kit-data.js", error: String((e && e.message) || e) }); }

// ui_kits/control-plane/kit-overlays.jsx
try { (() => {
/* ============================================================
   Control Plane UI kit — overlays.
   BrainDrawer · GatewayModal · CommandPalette
   ============================================================ */
const _NS3 = window.ControlPlaneDesignSystem_a21911;
const {
  Button: OBtn,
  IconButton: OIconBtn,
  StatusPill: OPill,
  RiskBadge: ORisk,
  EvidenceChip: OEvidence,
  MetaChip: OMeta
} = _NS3;
const OBadge = _NS3.Badge || (({
  children,
  mono,
  tone,
  variant,
  style = {}
}) => /*#__PURE__*/React.createElement("span", {
  style: {
    display: 'inline-flex',
    alignItems: 'center',
    gap: 4,
    height: 18,
    padding: '0 6px',
    borderRadius: 'var(--r-1)',
    background: 'var(--neutral-surface)',
    color: 'var(--text-secondary)',
    font: `var(--fw-medium) 11px/1 ${mono ? 'var(--font-mono)' : 'var(--font-sans)'}`,
    ...style
  }
}, children));
const {
  Ico: OIco,
  Eyebrow: OEye
} = window.KitShell;

/* ---------------- Project Brain drawer (hosts the co-pilot) ---------------- */
function BrainDrawer({
  open,
  onClose,
  openGateway,
  onExpand
}) {
  if (!open) return null;
  const Copilot = window.KitViews4 && window.KitViews4.ProjectBrainPage;
  return /*#__PURE__*/React.createElement(Overlay, {
    open: open,
    onClose: onClose,
    align: "right",
    width: 480
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      height: '100%',
      background: 'var(--surface-canvas)',
      borderLeft: '1px solid var(--brain-line)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      padding: '10px 12px',
      borderBottom: '1px solid var(--border-default)',
      background: 'var(--brain-surface)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--brain-ink)'
    }
  }, /*#__PURE__*/React.createElement(OIco, {
    n: "brain"
  })), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fw-semibold) var(--fs-sub) var(--font-sans)',
      color: 'var(--text-primary)'
    }
  }, "Project Brain"), /*#__PURE__*/React.createElement(OBadge, {
    tone: "brain",
    variant: "dot",
    style: {
      marginLeft: 4
    }
  }, "co-pilot"), /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: 'auto',
      display: 'flex',
      gap: 4
    }
  }, /*#__PURE__*/React.createElement(OIconBtn, {
    icon: /*#__PURE__*/React.createElement(OIco, {
      n: "brain-circuit",
      s: {
        width: 15,
        height: 15
      }
    }),
    "aria-label": "Memory & decisions",
    onClick: onExpand
  }), /*#__PURE__*/React.createElement(OIconBtn, {
    icon: /*#__PURE__*/React.createElement(OIco, {
      n: "x"
    }),
    "aria-label": "Close",
    onClick: onClose
  }))), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      minHeight: 0
    }
  }, Copilot ? /*#__PURE__*/React.createElement(Copilot, {
    openGateway: id => {
      openGateway && openGateway(id || 'edit');
    },
    drawer: true
  }) : null)));
}

/* ---------------- Action Gateway modal ---------------- */
function GatewayModal({
  open,
  item,
  onClose,
  onResolve
}) {
  if (!open || !item) return null;
  const isHigh = item.risk === 'high' || item.risk === 'critical';
  return /*#__PURE__*/React.createElement(Overlay, {
    open: open,
    onClose: onClose,
    align: "center",
    width: 520
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      background: 'var(--surface-card)',
      border: '1px solid var(--border-strong)',
      borderRadius: 'var(--r-4)',
      boxShadow: 'var(--elev-4)',
      overflow: 'hidden'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 9,
      padding: '13px 16px',
      borderBottom: '1px solid var(--border-default)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--accent-ink)'
    }
  }, /*#__PURE__*/React.createElement(OIco, {
    n: "shield-check"
  })), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fw-semibold) var(--fs-sub) var(--font-sans)'
    }
  }, "Action Gateway"), /*#__PURE__*/React.createElement(ORisk, {
    level: item.risk,
    style: {
      marginLeft: 4
    }
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: 'auto'
    }
  }, /*#__PURE__*/React.createElement(OIconBtn, {
    icon: /*#__PURE__*/React.createElement(OIco, {
      n: "x"
    }),
    "aria-label": "Close",
    onClick: onClose
  }))), /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '16px'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      marginBottom: 12
    }
  }, /*#__PURE__*/React.createElement(OPill, {
    status: "waiting-human",
    beacon: true
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fs-meta) var(--font-mono)',
      color: 'var(--text-muted)'
    }
  }, item.who)), /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fw-medium) var(--fs-body-lg)/1.4 var(--font-sans)',
      marginBottom: 6
    }
  }, item.title.split(item.cmd)[0], /*#__PURE__*/React.createElement("code", {
    style: {
      font: 'var(--fs-body) var(--font-mono)',
      background: 'var(--surface-sunken)',
      padding: '2px 6px',
      borderRadius: 4,
      color: isHigh ? 'var(--attention-ink)' : 'var(--accent-ink)'
    }
  }, item.cmd)), /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fs-label)/1.5 var(--font-sans)',
      color: 'var(--text-muted)',
      marginBottom: 14
    }
  }, "Worktree ", /*#__PURE__*/React.createElement("code", {
    style: {
      font: 'var(--fs-meta) var(--font-mono)'
    }
  }, item.wt), " \xB7 ", item.desc), /*#__PURE__*/React.createElement("div", {
    style: {
      background: 'var(--surface-sunken)',
      borderRadius: 'var(--r-2)',
      padding: '11px 12px',
      boxShadow: 'var(--elev-inset)',
      marginBottom: 4
    }
  }, /*#__PURE__*/React.createElement(OEye, {
    style: {
      marginBottom: 8
    }
  }, "What will happen"), item.conseq.map((c, i) => /*#__PURE__*/React.createElement(ConseqLine, {
    key: i,
    icon: c[0],
    text: c[1]
  })))), /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '12px 16px',
      borderTop: '1px solid var(--border-default)',
      display: 'flex',
      alignItems: 'center',
      gap: 8
    }
  }, /*#__PURE__*/React.createElement("label", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 7,
      font: 'var(--fs-label) var(--font-sans)',
      color: 'var(--text-muted)',
      marginRight: 'auto'
    }
  }, /*#__PURE__*/React.createElement("input", {
    type: "checkbox",
    style: {
      accentColor: 'var(--accent-solid)'
    }
  }), " Always allow in this project"), /*#__PURE__*/React.createElement(OBtn, {
    variant: "ghost",
    size: "md",
    onClick: () => onResolve(item, 'Denied')
  }, "Deny"), /*#__PURE__*/React.createElement(OBtn, {
    variant: isHigh ? 'danger-solid' : 'attention',
    size: "md",
    icon: /*#__PURE__*/React.createElement(OIco, {
      n: "check"
    }),
    kbd: "\u23CE",
    onClick: () => onResolve(item, 'Approved')
  }, isHigh ? 'Approve force-push' : 'Approve once'))));
}
function ConseqLine({
  icon,
  text
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      padding: '3px 0',
      font: 'var(--fs-label) var(--font-sans)',
      color: 'var(--text-secondary)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-faint)'
    }
  }, /*#__PURE__*/React.createElement(OIco, {
    n: icon,
    s: {
      width: 14,
      height: 14
    }
  })), text);
}

/* ---------------- Command palette (⌘K) ---------------- */
const CMDS = [{
  icon: 'brain',
  label: 'Ask Project Brain',
  hint: 'co-pilot',
  kbd: 'B',
  action: 'overlay:brain'
}, {
  icon: 'inbox',
  label: 'Open Task Inbox',
  hint: 'GitHub · Linear',
  kbd: 'P',
  action: 'overlay:tasks'
}, {
  icon: 'play',
  label: 'Start new session',
  hint: 'session',
  kbd: 'S',
  action: 'view:terminal'
}, {
  icon: 'package',
  label: 'Open Workflow Packs',
  hint: '/team-start',
  kbd: 'W',
  action: 'view:packs'
}, {
  icon: 'list-checks',
  label: 'Open Implementation Plan',
  hint: 'phases',
  action: 'view:plan'
}, {
  icon: 'users-round',
  label: 'Open Agent Team',
  hint: 'Phase 2 · cc-crew',
  action: 'view:team'
}, {
  icon: 'code-xml',
  label: 'Open Editor',
  hint: 'IDE',
  action: 'view:editor'
}, {
  icon: 'git-pull-request',
  label: 'Review pull request #84',
  hint: 'PR',
  action: 'view:code'
}, {
  icon: 'shield-check',
  label: 'Open Action Gateway queue',
  hint: 'approvals',
  action: 'overlay:gateway'
}, {
  icon: 'scroll-text',
  label: 'Open Audit Trail',
  hint: 'events',
  action: 'view:audit'
}, {
  icon: 'settings',
  label: 'Execution profiles & settings',
  hint: 'config',
  action: 'view:settings'
}];
function CommandPalette({
  open,
  onClose,
  onAction
}) {
  return /*#__PURE__*/React.createElement(Overlay, {
    open: open,
    onClose: onClose,
    align: "top",
    width: 560
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      background: 'var(--surface-card)',
      border: '1px solid var(--border-strong)',
      borderRadius: 'var(--r-4)',
      boxShadow: 'var(--elev-4)',
      overflow: 'hidden'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 10,
      padding: '12px 14px',
      borderBottom: '1px solid var(--border-default)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-faint)'
    }
  }, /*#__PURE__*/React.createElement(OIco, {
    n: "search"
  })), /*#__PURE__*/React.createElement("input", {
    autoFocus: true,
    placeholder: "Search objects or run a command\u2026",
    style: {
      flex: 1,
      background: 'transparent',
      border: 'none',
      outline: 'none',
      color: 'var(--text-primary)',
      font: 'var(--fs-body-lg) var(--font-sans)'
    }
  }), /*#__PURE__*/React.createElement("kbd", {
    style: {
      font: '10px var(--font-mono)',
      color: 'var(--text-faint)',
      border: '1px solid var(--border-default)',
      borderRadius: 3,
      padding: '1px 5px'
    }
  }, "esc")), /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '6px',
      maxHeight: 320,
      overflowY: 'auto'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fs-micro) var(--font-sans)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: 'var(--text-faint)',
      padding: '7px 9px 4px'
    }
  }, "Commands"), CMDS.map((c, i) => /*#__PURE__*/React.createElement("div", {
    key: i,
    onClick: () => {
      onAction && onAction(c.action);
      onClose();
    },
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 10,
      padding: '8px 9px',
      borderRadius: 'var(--r-2)',
      cursor: 'pointer',
      background: i === 0 ? 'var(--accent-surface)' : 'transparent'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: i === 0 ? 'var(--accent-ink)' : 'var(--text-muted)'
    }
  }, /*#__PURE__*/React.createElement(OIco, {
    n: c.icon,
    s: {
      width: 15,
      height: 15
    }
  })), /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1,
      font: 'var(--fs-body) var(--font-sans)',
      color: 'var(--text-primary)'
    }
  }, c.label), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fs-meta) var(--font-mono)',
      color: 'var(--text-faint)'
    }
  }, c.hint), c.kbd && /*#__PURE__*/React.createElement("kbd", {
    style: {
      font: '10px var(--font-mono)',
      color: 'var(--text-muted)',
      border: '1px solid var(--border-default)',
      borderRadius: 3,
      padding: '1px 5px'
    }
  }, c.kbd))))));
}

/* ---------------- Overlay shell ---------------- */
function Overlay({
  open,
  onClose,
  align = 'center',
  width = 480,
  children
}) {
  if (!open) return null;
  const pos = {
    center: {
      alignItems: 'center',
      justifyContent: 'center'
    },
    top: {
      alignItems: 'flex-start',
      justifyContent: 'center',
      paddingTop: '12vh'
    },
    right: {
      alignItems: 'stretch',
      justifyContent: 'flex-end'
    }
  }[align];
  return /*#__PURE__*/React.createElement("div", {
    onClick: onClose,
    style: {
      position: 'absolute',
      inset: 0,
      zIndex: 'var(--z-modal)',
      display: 'flex',
      background: 'var(--scrim)',
      backdropFilter: 'blur(2px)',
      ...pos
    }
  }, /*#__PURE__*/React.createElement("div", {
    onClick: e => e.stopPropagation(),
    style: {
      width: align === 'right' ? width : undefined,
      maxWidth: align === 'right' ? undefined : width,
      width: align === 'right' ? width : '100%',
      margin: align === 'right' ? 0 : '0 16px',
      animation: align === 'right' ? 'cp-slide-in 0.24s var(--ease-out)' : 'cp-pop-in 0.18s var(--ease-out)'
    }
  }, children));
}

/* ---------------- Inspector drawer (graph node / object) ---------------- */
function InspectorDrawer({
  node,
  onClose,
  onOpen
}) {
  if (!node) return null;
  const kindIcon = (window.NODE_KIND_ICON || {})[node.kind] || 'box';
  const tint = node.kind === 'brain' ? 'var(--brain-ink)' : node.kind === 'team' ? 'var(--teal-ink)' : 'var(--text-secondary)';
  const surf = node.kind === 'brain' ? 'var(--brain-surface)' : node.kind === 'team' ? 'var(--teal-surface)' : 'var(--surface-active)';
  return /*#__PURE__*/React.createElement(Overlay, {
    open: !!node,
    onClose: onClose,
    align: "right",
    width: 340
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      height: '100%',
      background: 'var(--surface-panel)',
      borderLeft: '1px solid var(--border-strong)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 10,
      padding: '12px 14px',
      borderBottom: '1px solid var(--border-default)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      width: 28,
      height: 28,
      flex: 'none',
      borderRadius: 'var(--r-2)',
      background: surf,
      color: tint,
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center'
    }
  }, /*#__PURE__*/React.createElement(OIco, {
    n: kindIcon,
    s: {
      width: 15,
      height: 15
    }
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      minWidth: 0,
      flex: 1
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fw-semibold) var(--fs-body) var(--font-sans)',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap'
    }
  }, node.title), /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fs-micro) var(--font-mono)',
      color: 'var(--text-faint)',
      textTransform: 'uppercase',
      letterSpacing: 'var(--tracking-caps)'
    }
  }, "Inspector \xB7 ", node.kind)), /*#__PURE__*/React.createElement(OIconBtn, {
    icon: /*#__PURE__*/React.createElement(OIco, {
      n: "x"
    }),
    "aria-label": "Close",
    onClick: onClose
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '12px 14px',
      borderBottom: '1px solid var(--border-subtle)',
      display: 'flex',
      alignItems: 'center',
      gap: 8
    }
  }, /*#__PURE__*/React.createElement(OPill, {
    status: node.status,
    beacon: node.beacon
  }), node.subtitle && /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fs-meta) var(--font-mono)',
      color: 'var(--text-muted)'
    }
  }, node.subtitle)), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      overflowY: 'auto',
      padding: '12px 14px'
    }
  }, /*#__PURE__*/React.createElement(OEye, {
    style: {
      marginBottom: 8
    }
  }, "Details"), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column'
    }
  }, Object.entries(node.detail || {}).map(([k, v]) => /*#__PURE__*/React.createElement("div", {
    key: k,
    style: {
      display: 'flex',
      alignItems: 'baseline',
      gap: 12,
      padding: '7px 0',
      borderBottom: '1px solid var(--border-subtle)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 'none',
      width: 84,
      font: 'var(--fs-meta) var(--font-sans)',
      color: 'var(--text-faint)'
    }
  }, k), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fs-meta) var(--font-mono)',
      color: 'var(--text-secondary)',
      textAlign: 'right',
      marginLeft: 'auto'
    }
  }, v))))), node.open && /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '12px 14px',
      borderTop: '1px solid var(--border-default)',
      display: 'flex',
      gap: 8
    }
  }, /*#__PURE__*/React.createElement(OBtn, {
    variant: "primary",
    size: "md",
    full: true,
    icon: /*#__PURE__*/React.createElement(OIco, {
      n: node.open.team ? 'terminal' : 'arrow-right',
      s: {
        width: 14,
        height: 14
      }
    }),
    onClick: () => {
      onClose();
      onOpen && onOpen(node.open);
    }
  }, node.open.label))));
}
window.KitOverlays = {
  BrainDrawer,
  GatewayModal,
  CommandPalette,
  InspectorDrawer
};
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/control-plane/kit-overlays.jsx", error: String((e && e.message) || e) }); }

// ui_kits/control-plane/kit-plan.jsx
try { (() => {
/* ============================================================
   Control Plane UI kit — Plan View (implementation plan).
   Phase → Track → PlanTask hierarchy with dispatch.
   ============================================================ */
const _NS8 = window.ControlPlaneDesignSystem_a21911;
const {
  Button: PBtn,
  IconButton: PIconBtn,
  StatusPill: PPill,
  MetaChip: PMeta
} = _NS8;
const PBadge = _NS8.Badge || (({
  children,
  mono,
  style = {}
}) => /*#__PURE__*/React.createElement("span", {
  style: {
    font: `var(--fw-medium) 11px/1 ${mono ? 'var(--font-mono)' : 'var(--font-sans)'}`,
    ...style
  }
}, children));
const {
  Ico: PIco,
  Eyebrow: PEye
} = window.KitShell;
const KD8 = window.KIT;
const {
  useState: useS8
} = React;
const TASK_STATUS = {
  completed: {
    pill: 'completed',
    label: 'Done'
  },
  'in-progress': {
    pill: 'running',
    label: 'In progress'
  },
  ready: {
    pill: 'idle',
    label: 'Ready'
  },
  todo: {
    pill: 'idle',
    label: 'Todo'
  },
  backlog: {
    pill: 'archived',
    label: 'Backlog'
  }
};
const PHASE_STATUS = {
  completed: 'completed',
  active: 'running',
  backlog: 'archived'
};
function PlanView({
  onDispatch,
  openBrain
}) {
  const plan = KD8.plan;
  const all = plan.phases.flatMap(ph => ph.tracks.flatMap(t => t.tasks));
  const done = all.filter(t => t.status === 'completed').length;
  const pct = Math.round(done / all.length * 100);
  const [openPhases, setOpenPhases] = useS8({
    p1: false,
    p2: true,
    p3: true
  });
  React.useEffect(() => {
    const t = setTimeout(() => window.lucide && window.lucide.createIcons(), 24);
    return () => clearTimeout(t);
  }, [openPhases]);
  return /*#__PURE__*/React.createElement("div", {
    style: {
      height: '100%',
      overflowY: 'auto',
      background: 'var(--surface-canvas)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'sticky',
      top: 0,
      zIndex: 5,
      padding: '14px 16px',
      background: 'var(--surface-canvas)',
      borderBottom: '1px solid var(--border-subtle)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 10
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-secondary)'
    }
  }, /*#__PURE__*/React.createElement(PIco, {
    n: "list-checks"
  })), /*#__PURE__*/React.createElement("h1", {
    style: {
      margin: 0,
      font: 'var(--fw-semibold) var(--fs-h3)/1 var(--font-sans)'
    }
  }, "Implementation plan"), /*#__PURE__*/React.createElement(PMeta, {
    icon: /*#__PURE__*/React.createElement(PIco, {
      n: "file-text",
      s: {
        width: 12,
        height: 12
      }
    })
  }, plan.source), /*#__PURE__*/React.createElement(PBadge, {
    tone: "teal",
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      height: 18,
      padding: '0 6px',
      borderRadius: 'var(--r-1)',
      background: 'var(--teal-surface)',
      color: 'var(--teal-ink)'
    }
  }, "parser: ", plan.parser), /*#__PURE__*/React.createElement("div", {
    style: {
      marginLeft: 'auto',
      display: 'flex',
      gap: 6
    }
  }, /*#__PURE__*/React.createElement(PBtn, {
    variant: "ghost",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(PIco, {
      n: "brain"
    }),
    onClick: openBrain
  }, "Ask Brain"))), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 10,
      marginTop: 11
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      maxWidth: 320,
      height: 6,
      borderRadius: 999,
      background: 'var(--cap-track)',
      overflow: 'hidden'
    }
  }, /*#__PURE__*/React.createElement("i", {
    style: {
      display: 'block',
      height: '100%',
      width: pct + '%',
      background: 'var(--success-solid)'
    }
  })), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fs-meta) var(--font-mono)',
      color: 'var(--text-muted)'
    }
  }, done, "/", all.length, " tasks \xB7 ", pct, "%"))), /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '14px 16px',
      maxWidth: 820,
      display: 'flex',
      flexDirection: 'column',
      gap: 10
    }
  }, plan.phases.map(ph => {
    const open = openPhases[ph.id];
    const ptasks = ph.tracks.flatMap(t => t.tasks);
    const pdone = ptasks.filter(t => t.status === 'completed').length;
    return /*#__PURE__*/React.createElement("div", {
      key: ph.id,
      style: {
        border: '1px solid var(--border-default)',
        borderRadius: 'var(--r-3)',
        overflow: 'hidden',
        background: 'var(--surface-card)'
      }
    }, /*#__PURE__*/React.createElement("div", {
      onClick: () => setOpenPhases(s => ({
        ...s,
        [ph.id]: !s[ph.id]
      })),
      role: "button",
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 9,
        width: '100%',
        padding: '11px 13px',
        background: ph.status === 'active' ? 'var(--accent-surface)' : 'transparent',
        cursor: 'pointer',
        textAlign: 'left'
      }
    }, /*#__PURE__*/React.createElement(PIco, {
      n: open ? 'chevron-down' : 'chevron-right',
      s: {
        width: 15,
        height: 15,
        color: 'var(--text-faint)'
      }
    }), /*#__PURE__*/React.createElement("span", {
      style: {
        font: 'var(--fw-semibold) var(--fs-body) var(--font-sans)',
        color: ph.status === 'active' ? 'var(--accent-ink)' : 'var(--text-primary)'
      }
    }, ph.name), /*#__PURE__*/React.createElement(PPill, {
      status: PHASE_STATUS[ph.status],
      size: "xs"
    }), /*#__PURE__*/React.createElement("span", {
      style: {
        marginLeft: 'auto',
        font: 'var(--fs-meta) var(--font-mono)',
        color: 'var(--text-muted)'
      }
    }, pdone, "/", ptasks.length), ph.status !== 'completed' && /*#__PURE__*/React.createElement(PBtn, {
      variant: "ghost",
      size: "xs",
      icon: /*#__PURE__*/React.createElement(PIco, {
        n: "users-round",
        s: {
          width: 12,
          height: 12
        }
      }),
      onClick: e => {
        e.stopPropagation();
        onDispatch({
          source: 'plan',
          id: ph.name.split(' · ')[0],
          title: ph.name,
          harness: 'claude-code',
          profile: 'Claude Team Work',
          branch: 'team/phase',
          planTask: ph.name
        });
      }
    }, "Start team")), open && /*#__PURE__*/React.createElement("div", {
      style: {
        padding: '4px 10px 10px'
      }
    }, ph.tracks.map(tr => /*#__PURE__*/React.createElement("div", {
      key: tr.id,
      style: {
        marginTop: 6
      }
    }, /*#__PURE__*/React.createElement(PEye, {
      style: {
        padding: '4px 6px 7px'
      }
    }, tr.name), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        flexDirection: 'column',
        gap: 5
      }
    }, tr.tasks.map(task => /*#__PURE__*/React.createElement(PlanTaskRow, {
      key: task.id,
      task: task,
      onDispatch: onDispatch,
      openBrain: openBrain
    })))))));
  })));
}
function PlanTaskRow({
  task,
  onDispatch,
  openBrain
}) {
  const st = TASK_STATUS[task.status] || TASK_STATUS.todo;
  const dispatchable = task.status !== 'completed';
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 10,
      padding: '9px 10px',
      borderRadius: 'var(--r-2)',
      border: '1px solid var(--border-subtle)',
      background: 'var(--surface-panel)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fs-meta) var(--font-mono)',
      color: 'var(--text-faint)',
      width: 30,
      flex: 'none'
    }
  }, task.id), /*#__PURE__*/React.createElement(PPill, {
    status: st.pill,
    size: "xs",
    label: st.label
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      minWidth: 0
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fw-medium) var(--fs-label)/1.3 var(--font-sans)',
      color: 'var(--text-primary)',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap'
    }
  }, task.title), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      marginTop: 3,
      font: 'var(--fs-micro) var(--font-mono)',
      color: 'var(--text-faint)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    title: "acceptance criteria"
  }, /*#__PURE__*/React.createElement(PIco, {
    n: "check-check",
    s: {
      width: 11,
      height: 11
    }
  }), " ", task.ac, " AC"), task.files > 0 && /*#__PURE__*/React.createElement("span", {
    title: "files"
  }, "\xB7 ", task.files, " files"), task.anchors && task.anchors.map(a => /*#__PURE__*/React.createElement("span", {
    key: a,
    style: {
      color: 'var(--brain-ink)'
    }
  }, "\xB7 \u2693 ", a.split('#')[1] || a)), task.session && /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--live-ink)'
    }
  }, "\xB7 \u25CF in session"), task.team && /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--teal-ink)'
    }
  }, "\xB7 team"))), task.task && /*#__PURE__*/React.createElement(PMeta, {
    tone: task.task.tone === 'linear' ? 'linear' : task.task.tone === 'github' ? 'github' : 'accent',
    mono: false
  }, task.task.id), dispatchable ? /*#__PURE__*/React.createElement(PBtn, {
    variant: "secondary",
    size: "xs",
    icon: /*#__PURE__*/React.createElement(PIco, {
      n: "play",
      s: {
        width: 12,
        height: 12
      }
    }),
    onClick: () => onDispatch({
      source: 'plan',
      id: task.id,
      title: task.title,
      harness: 'claude-code',
      profile: 'Claude Max Main',
      branch: task.task ? null : 'agent/' + task.id,
      planTask: task.id
    })
  }, "Start") : /*#__PURE__*/React.createElement(PIconBtn, {
    icon: /*#__PURE__*/React.createElement(PIco, {
      n: "git-pull-request",
      s: {
        width: 14,
        height: 14
      }
    }),
    size: "sm",
    "aria-label": "View PR"
  }));
}
window.KitPlan = {
  PlanView
};
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/control-plane/kit-plan.jsx", error: String((e && e.message) || e) }); }

// ui_kits/control-plane/kit-shell.jsx
try { (() => {
/* ============================================================
   Control Plane UI kit — interactive desktop shell.
   Composes the design-system components (window namespace) + KIT data.
   ============================================================ */
const NS = window.ControlPlaneDesignSystem_a21911;
const {
  Button,
  IconButton,
  StatusPill,
  RiskBadge,
  UsageMeter,
  AttentionMarker,
  HarnessBadge,
  ProfileBadge,
  MetaChip,
  SessionRow,
  EvidenceChip,
  GraphNode,
  DiffHunk
} = NS;
const {
  useState,
  useEffect
} = React;
const K = window.KIT;
const Ico = ({
  n,
  s
}) => /*#__PURE__*/React.createElement("i", {
  "data-lucide": n,
  style: {
    width: 16,
    height: 16,
    ...s
  }
});
const Eyebrow = ({
  children,
  style
}) => /*#__PURE__*/React.createElement("div", {
  style: {
    font: 'var(--fw-semibold) var(--fs-micro)/1 var(--font-sans)',
    letterSpacing: 'var(--tracking-caps)',
    textTransform: 'uppercase',
    color: 'var(--text-faint)',
    ...style
  }
}, children);

/* ---------------- Top bar ---------------- */
function ProjectSwitcher({
  proj,
  project,
  onSelect
}) {
  const [open, setOpen] = useState(false);
  useEffect(() => {
    if (!open) return;
    const close = () => setOpen(false);
    window.addEventListener('click', close);
    return () => window.removeEventListener('click', close);
  }, [open]);
  const items = [{
    id: 'all',
    name: 'All projects',
    repo: `${K.projects.length} projects`,
    active: 0,
    waiting: 0,
    all: true
  }, ...K.projects];
  const wfTone = {
    active: 'var(--teal-ink)',
    drift: 'var(--warning-ink)',
    'needs-personalization': 'var(--caution-ink)',
    none: 'var(--text-faint)'
  };
  return /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'relative'
    },
    onClick: e => e.stopPropagation()
  }, /*#__PURE__*/React.createElement("button", {
    onClick: () => setOpen(o => !o),
    title: "Switch project",
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 7,
      height: 28,
      padding: '0 9px',
      borderRadius: 'var(--r-2)',
      border: `1px solid ${open ? 'var(--border-strong)' : 'var(--border-default)'}`,
      background: 'var(--surface-input)',
      cursor: 'pointer'
    }
  }, /*#__PURE__*/React.createElement(Ico, {
    n: proj.all ? 'layout-grid' : 'folder-git-2',
    s: {
      width: 14,
      height: 14,
      color: 'var(--text-muted)'
    }
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fw-medium) var(--fs-label)/1 var(--font-sans)',
      color: 'var(--text-primary)',
      maxWidth: 220,
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap'
    }
  }, proj.name), /*#__PURE__*/React.createElement(Ico, {
    n: "chevrons-up-down",
    s: {
      width: 13,
      height: 13,
      color: 'var(--text-faint)'
    }
  })), open && /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      top: 32,
      left: 0,
      zIndex: 'var(--z-popover)',
      width: 320,
      background: 'var(--surface-card)',
      border: '1px solid var(--border-strong)',
      borderRadius: 'var(--r-3)',
      boxShadow: 'var(--elev-3)',
      overflow: 'hidden',
      padding: '5px',
      animation: 'cp-pop-in 0.14s var(--ease-out)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fw-semibold) var(--fs-micro) var(--font-sans)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: 'var(--text-faint)',
      padding: '6px 8px 4px'
    }
  }, "Switch project"), items.map(p => {
    const on = project === p.id;
    return /*#__PURE__*/React.createElement("button", {
      key: p.id,
      onClick: () => {
        onSelect(p.id);
        setOpen(false);
      },
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 9,
        width: '100%',
        padding: '7px 8px',
        borderRadius: 'var(--r-2)',
        border: 'none',
        cursor: 'pointer',
        textAlign: 'left',
        background: on ? 'var(--accent-surface)' : 'transparent'
      }
    }, /*#__PURE__*/React.createElement(Ico, {
      n: p.all ? 'layout-grid' : 'folder-git-2',
      s: {
        width: 15,
        height: 15,
        color: on ? 'var(--accent-ink)' : 'var(--text-muted)'
      }
    }), /*#__PURE__*/React.createElement("div", {
      style: {
        flex: 1,
        minWidth: 0
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        font: 'var(--fw-medium) var(--fs-label) var(--font-sans)',
        color: on ? 'var(--accent-ink)' : 'var(--text-primary)',
        overflow: 'hidden',
        textOverflow: 'ellipsis',
        whiteSpace: 'nowrap'
      }
    }, p.name), /*#__PURE__*/React.createElement("div", {
      style: {
        font: 'var(--fs-micro) var(--font-mono)',
        color: 'var(--text-faint)'
      }
    }, p.repo)), !p.all && p.waiting > 0 && /*#__PURE__*/React.createElement("span", {
      title: "waiting",
      style: {
        width: 7,
        height: 7,
        borderRadius: 999,
        background: 'var(--attention-solid)'
      }
    }), !p.all && p.active > 0 && /*#__PURE__*/React.createElement("span", {
      style: {
        font: '10px var(--font-mono)',
        color: 'var(--text-muted)'
      }
    }, p.active), !p.all && /*#__PURE__*/React.createElement("span", {
      title: 'workflow: ' + p.workflow,
      style: {
        width: 6,
        height: 6,
        borderRadius: 2,
        background: wfTone[p.workflow] || 'var(--text-faint)'
      }
    }), on && /*#__PURE__*/React.createElement(Ico, {
      n: "check",
      s: {
        width: 14,
        height: 14,
        color: 'var(--accent-ink)'
      }
    }));
  })));
}
function TopBar({
  view,
  setView,
  proj,
  project,
  onSelectProject,
  onBack,
  canBack,
  onForward,
  canForward,
  onBrain,
  onGateway,
  onPalette,
  onHumanInput,
  onTasks,
  waiting
}) {
  proj = proj || {
    name: 'Project',
    repo: '',
    active: 0,
    prs: 0,
    brain: 'ready'
  };
  return /*#__PURE__*/React.createElement("header", {
    style: {
      gridArea: 'top',
      display: 'flex',
      alignItems: 'center',
      gap: 10,
      padding: '0 12px',
      height: 'var(--shell-topbar-h)',
      background: 'var(--surface-panel)',
      borderBottom: '1px solid var(--border-default)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 6,
      alignItems: 'center',
      paddingRight: 2
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'flex',
      gap: 6
    }
  }, ['#3b3b40', '#3b3b40', '#3b3b40'].map((c, i) => /*#__PURE__*/React.createElement("span", {
    key: i,
    style: {
      width: 11,
      height: 11,
      borderRadius: 999,
      background: c,
      border: '1px solid var(--border-strong)'
    }
  })))), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 1
    }
  }, /*#__PURE__*/React.createElement("button", {
    onClick: onBack,
    disabled: !canBack,
    title: "Back",
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      width: 26,
      height: 26,
      borderRadius: 'var(--r-2)',
      border: 'none',
      background: 'transparent',
      cursor: canBack ? 'pointer' : 'default',
      color: canBack ? 'var(--text-secondary)' : 'var(--text-faint)',
      opacity: canBack ? 1 : 0.4
    }
  }, /*#__PURE__*/React.createElement(Ico, {
    n: "arrow-left",
    s: {
      width: 16,
      height: 16
    }
  })), /*#__PURE__*/React.createElement("button", {
    onClick: onForward,
    disabled: !canForward,
    title: "Forward",
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      width: 26,
      height: 26,
      borderRadius: 'var(--r-2)',
      border: 'none',
      background: 'transparent',
      cursor: canForward ? 'pointer' : 'default',
      color: canForward ? 'var(--text-secondary)' : 'var(--text-faint)',
      opacity: canForward ? 1 : 0.4
    }
  }, /*#__PURE__*/React.createElement(Ico, {
    n: "arrow-right",
    s: {
      width: 16,
      height: 16
    }
  }))), /*#__PURE__*/React.createElement("span", {
    style: {
      width: 1,
      height: 18,
      background: 'var(--border-default)'
    }
  }), /*#__PURE__*/React.createElement(ProjectSwitcher, {
    proj: proj,
    project: project,
    onSelect: onSelectProject
  }), proj.repo && /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fs-meta) var(--font-mono)',
      color: 'var(--text-faint)'
    }
  }, proj.repo), /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 11,
      marginLeft: 2,
      font: 'var(--fs-meta) var(--font-mono)',
      color: 'var(--text-muted)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    title: "active sessions",
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 4
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      width: 6,
      height: 6,
      borderRadius: 999,
      background: 'var(--live-solid)'
    }
  }), proj.active), /*#__PURE__*/React.createElement("span", {
    title: "open PRs",
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 4
    }
  }, /*#__PURE__*/React.createElement(Ico, {
    n: "git-pull-request",
    s: {
      width: 12,
      height: 12
    }
  }), proj.prs), waiting > 0 && /*#__PURE__*/React.createElement("span", {
    title: "waiting on you",
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 4,
      color: 'var(--attention-ink)'
    }
  }, "\u25C6 ", waiting)), /*#__PURE__*/React.createElement("button", {
    onClick: onPalette,
    style: {
      marginLeft: 'auto',
      width: 320,
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      height: 28,
      padding: '0 10px',
      borderRadius: 'var(--r-2)',
      background: 'var(--surface-input)',
      border: '1px solid var(--border-default)',
      color: 'var(--text-muted)',
      cursor: 'text',
      font: 'var(--fs-label) var(--font-sans)'
    }
  }, /*#__PURE__*/React.createElement(Ico, {
    n: "search",
    s: {
      width: 14,
      height: 14
    }
  }), " Search or run a command\u2026", /*#__PURE__*/React.createElement("kbd", {
    style: {
      marginLeft: 'auto',
      padding: '1px 5px',
      borderRadius: 3,
      background: 'var(--surface-active)',
      border: '1px solid var(--border-default)',
      font: '10px var(--font-mono)'
    }
  }, "\u2318K")), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 6
    }
  }, /*#__PURE__*/React.createElement("span", {
    title: "Local runtime healthy",
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 5,
      padding: '0 8px',
      height: 24,
      borderRadius: 999,
      border: '1px solid var(--success-line)',
      background: 'var(--success-surface)',
      font: '10px var(--font-mono)',
      color: 'var(--success-ink)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      width: 6,
      height: 6,
      borderRadius: 999,
      background: 'var(--success-solid)'
    }
  }), " runtime"), /*#__PURE__*/React.createElement(IconButton, {
    label: "Task Inbox",
    onClick: onTasks
  }, /*#__PURE__*/React.createElement(Ico, {
    n: "inbox"
  })), /*#__PURE__*/React.createElement(IconButton, {
    label: "Human input queue",
    badge: waiting,
    onClick: onHumanInput
  }, /*#__PURE__*/React.createElement(Ico, {
    n: "bell"
  })), /*#__PURE__*/React.createElement(IconButton, {
    label: "Action Gateway",
    onClick: onGateway
  }, /*#__PURE__*/React.createElement(Ico, {
    n: "shield-check"
  })), /*#__PURE__*/React.createElement(Button, {
    variant: "brain",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(Ico, {
      n: "brain"
    }),
    onClick: onBrain
  }, "Brain"), /*#__PURE__*/React.createElement(IconButton, {
    label: "Settings",
    onClick: () => setView('settings')
  }, /*#__PURE__*/React.createElement(Ico, {
    n: "settings"
  }))));
}

/* ---------------- Sidebar ---------------- */
const VIEWS = [{
  id: 'command',
  label: 'Command Center',
  icon: 'layout-dashboard'
}, {
  id: 'graph',
  label: 'Project Graph',
  icon: 'workflow'
}, {
  id: 'plan',
  label: 'Plan',
  icon: 'list-checks'
}, {
  id: 'editor',
  label: 'Editor',
  icon: 'code-xml'
}, {
  id: 'terminal',
  label: 'Session Terminal',
  icon: 'terminal'
}, {
  id: 'code',
  label: 'Code / Diff Review',
  icon: 'file-diff'
}];
const PLATFORM_VIEWS = [{
  id: 'packs',
  label: 'Workflow Packs',
  icon: 'package'
}, {
  id: 'audit',
  label: 'Audit Trail',
  icon: 'scroll-text'
}];
function NavBtn({
  v,
  view,
  setView
}) {
  const on = view === v.id;
  const brain = v.id === 'brain';
  return /*#__PURE__*/React.createElement("button", {
    onClick: () => setView(v.id),
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      width: '100%',
      height: 28,
      padding: '0 8px',
      borderRadius: 'var(--r-2)',
      border: 'none',
      cursor: 'pointer',
      font: 'var(--fw-medium) var(--fs-label) var(--font-sans)',
      textAlign: 'left',
      background: on ? brain ? 'var(--brain-surface)' : 'var(--accent-surface)' : 'transparent',
      color: on ? brain ? 'var(--brain-ink)' : 'var(--accent-ink)' : 'var(--text-secondary)'
    }
  }, /*#__PURE__*/React.createElement(Ico, {
    n: v.icon
  }), " ", v.label);
}
function Sidebar({
  view,
  setView,
  sel,
  setSel,
  waiting = 0,
  onHumanInput,
  onTasks,
  overrides = {}
}) {
  const openSession = s => {
    setSel(s.id);
    setView(s.team ? 'team' : 'terminal');
  };
  return /*#__PURE__*/React.createElement("nav", {
    style: {
      gridArea: 'side',
      display: 'flex',
      flexDirection: 'column',
      background: 'var(--surface-panel)',
      borderRight: '1px solid var(--border-default)',
      overflow: 'hidden'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      overflowY: 'auto',
      padding: '10px 10px 10px'
    }
  }, /*#__PURE__*/React.createElement(Eyebrow, {
    style: {
      padding: '0 6px 6px'
    }
  }, "Workspace"), /*#__PURE__*/React.createElement(ProjectsNav, {
    view: view,
    setView: setView,
    sel: sel,
    openSession: openSession,
    overrides: overrides
  }), VIEWS.map(v => /*#__PURE__*/React.createElement(NavBtn, {
    key: v.id,
    v: v,
    view: view,
    setView: setView
  })), /*#__PURE__*/React.createElement("button", {
    onClick: onHumanInput,
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      width: '100%',
      height: 28,
      padding: '0 8px',
      borderRadius: 'var(--r-2)',
      border: 'none',
      cursor: 'pointer',
      font: 'var(--fw-medium) var(--fs-label) var(--font-sans)',
      background: 'transparent',
      color: waiting > 0 ? 'var(--attention-ink)' : 'var(--text-faint)'
    }
  }, /*#__PURE__*/React.createElement(Ico, {
    n: "circle-alert"
  }), " Human Input", waiting > 0 && /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: 'auto',
      padding: '0 6px',
      height: 16,
      display: 'inline-flex',
      alignItems: 'center',
      borderRadius: 999,
      background: 'var(--attention-solid)',
      color: 'var(--attention-on-solid)',
      font: '10px var(--font-mono)'
    }
  }, waiting)), /*#__PURE__*/React.createElement("button", {
    onClick: onTasks,
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      width: '100%',
      height: 28,
      padding: '0 8px',
      borderRadius: 'var(--r-2)',
      border: 'none',
      cursor: 'pointer',
      font: 'var(--fw-medium) var(--fs-label) var(--font-sans)',
      background: 'transparent',
      color: 'var(--text-secondary)',
      textAlign: 'left'
    }
  }, /*#__PURE__*/React.createElement(Ico, {
    n: "inbox"
  }), " Task Inbox", /*#__PURE__*/React.createElement("kbd", {
    style: {
      marginLeft: 'auto',
      font: '9px var(--font-mono)',
      color: 'var(--text-faint)',
      border: '1px solid var(--border-default)',
      borderRadius: 3,
      padding: '0 4px'
    }
  }, "\u2318\u21E7P")), /*#__PURE__*/React.createElement(Eyebrow, {
    style: {
      padding: '12px 6px 6px'
    }
  }, "Platform"), PLATFORM_VIEWS.map(v => /*#__PURE__*/React.createElement(NavBtn, {
    key: v.id,
    v: v,
    view: view,
    setView: setView
  }))));
}

/* Projects: expandable tree (chevron) + click-into page (label). */
function ProjectsNav({
  view,
  setView,
  sel,
  openSession,
  overrides
}) {
  const [exp, setExp] = useState(true);
  const on = view === 'projects';
  return /*#__PURE__*/React.createElement("div", {
    style: {
      marginBottom: 2
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      height: 28,
      borderRadius: 'var(--r-2)',
      background: on ? 'var(--accent-surface)' : 'transparent'
    }
  }, /*#__PURE__*/React.createElement("button", {
    onClick: () => setExp(e => !e),
    title: exp ? 'Collapse' : 'Expand',
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      width: 24,
      height: 28,
      border: 'none',
      background: 'transparent',
      cursor: 'pointer',
      color: 'var(--text-faint)',
      flex: 'none'
    }
  }, /*#__PURE__*/React.createElement(Ico, {
    n: exp ? 'chevron-down' : 'chevron-right',
    s: {
      width: 14,
      height: 14
    }
  })), /*#__PURE__*/React.createElement("button", {
    onClick: () => setView('projects'),
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      flex: 1,
      height: 28,
      padding: '0 8px 0 2px',
      border: 'none',
      background: 'transparent',
      cursor: 'pointer',
      font: 'var(--fw-medium) var(--fs-label) var(--font-sans)',
      textAlign: 'left',
      color: on ? 'var(--accent-ink)' : 'var(--text-secondary)'
    }
  }, /*#__PURE__*/React.createElement(Ico, {
    n: "layout-grid"
  }), " Projects", /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: 'auto',
      font: '10px var(--font-mono)',
      color: 'var(--text-faint)'
    }
  }, K.projects.length))), exp && /*#__PURE__*/React.createElement("div", {
    style: {
      margin: '2px 0 4px',
      paddingLeft: 4
    }
  }, K.projects.map(p => /*#__PURE__*/React.createElement(ProjectGroup, {
    key: p.id,
    p: p,
    open: p.id === 'cp',
    sessions: K.sessions.filter(s => s.proj === p.id),
    sel: sel,
    overrides: overrides,
    setSel: id => {
      const s = K.sessions.find(x => x.id === id);
      if (s) openSession(s);
    }
  }))));
}
function ProjectGroup({
  p,
  open,
  sessions,
  sel,
  setSel,
  overrides = {}
}) {
  const [exp, setExp] = useState(open);
  const wfTone = {
    active: 'var(--teal-ink)',
    drift: 'var(--warning-ink)',
    'needs-personalization': 'var(--caution-ink)',
    none: 'var(--text-faint)'
  }[p.workflow];
  return /*#__PURE__*/React.createElement("div", {
    style: {
      marginBottom: 2
    }
  }, /*#__PURE__*/React.createElement("button", {
    onClick: () => setExp(!exp),
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 6,
      width: '100%',
      padding: '5px 6px',
      borderRadius: 'var(--r-2)',
      border: 'none',
      background: 'transparent',
      cursor: 'pointer',
      textAlign: 'left'
    }
  }, /*#__PURE__*/React.createElement(Ico, {
    n: exp ? 'chevron-down' : 'chevron-right',
    s: {
      color: 'var(--text-faint)'
    }
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1,
      font: 'var(--fw-medium) var(--fs-label) var(--font-sans)',
      color: 'var(--text-primary)',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap'
    }
  }, p.name), p.waiting > 0 && /*#__PURE__*/React.createElement("span", {
    title: "waiting on human",
    style: {
      width: 7,
      height: 7,
      borderRadius: 999,
      background: 'var(--attention-solid)'
    }
  }), p.active > 0 && /*#__PURE__*/React.createElement("span", {
    style: {
      font: '10px var(--font-mono)',
      color: 'var(--text-muted)'
    }
  }, p.active), /*#__PURE__*/React.createElement("span", {
    title: 'workflow: ' + p.workflow,
    style: {
      width: 6,
      height: 6,
      borderRadius: 2,
      background: wfTone
    }
  })), exp && sessions.map(s => /*#__PURE__*/React.createElement("button", {
    key: s.id,
    onClick: () => setSel(s.id),
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 7,
      width: '100%',
      padding: '4px 6px 4px 22px',
      borderRadius: 'var(--r-2)',
      border: 'none',
      cursor: 'pointer',
      textAlign: 'left',
      background: sel === s.id ? 'var(--surface-active)' : 'transparent'
    }
  }, /*#__PURE__*/React.createElement(AttentionMarker, {
    level: {
      'waiting-human': 5,
      failed: 4,
      running: 2,
      active: 1
    }[overrides[s.id] || s.status] || 0,
    variant: "dot"
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1,
      minWidth: 0,
      font: 'var(--fs-meta) var(--font-sans)',
      color: sel === s.id ? 'var(--text-primary)' : 'var(--text-secondary)',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap'
    }
  }, s.title), s.team && /*#__PURE__*/React.createElement("span", {
    title: "Agent team \u2014 open team view",
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 3,
      flex: 'none',
      height: 15,
      padding: '0 5px',
      borderRadius: 'var(--r-1)',
      background: 'var(--teal-surface)',
      border: '1px solid var(--teal-line)',
      color: 'var(--teal-ink)',
      font: 'var(--fw-semibold) 9px/1 var(--font-sans)'
    }
  }, /*#__PURE__*/React.createElement(Ico, {
    n: "users-round",
    s: {
      width: 10,
      height: 10
    }
  }), " Team"))));
}

/* ---------------- Bottom event / activity dock (drawer-first) ---------------- */
const DOCK_ICON = {
  approval: 'shield-check',
  gateway: 'shield-x',
  git: 'git-commit-horizontal',
  session: 'circle-dot',
  brain: 'brain',
  pr: 'git-pull-request',
  workflow: 'workflow',
  profile: 'key-round'
};
function EventDock({
  project,
  projectName,
  open,
  onToggle,
  onOpenAudit
}) {
  const events = (K.audit || []).filter(e => project === 'all' || e.proj === project);
  const latest = events[0];
  return /*#__PURE__*/React.createElement("div", {
    style: {
      gridArea: 'dock',
      background: 'var(--surface-panel)',
      borderTop: '1px solid var(--border-default)',
      display: 'flex',
      flexDirection: 'column',
      height: open ? 'var(--shell-timeline-h)' : 'var(--shell-statusbar-h)',
      transition: 'height var(--dur-3) var(--ease-standard)',
      overflow: 'hidden'
    }
  }, /*#__PURE__*/React.createElement("button", {
    onClick: onToggle,
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 10,
      height: 'var(--shell-statusbar-h)',
      flex: 'none',
      padding: '0 12px',
      border: 'none',
      background: 'transparent',
      cursor: 'pointer',
      textAlign: 'left',
      width: '100%'
    }
  }, /*#__PURE__*/React.createElement(Ico, {
    n: open ? 'chevron-down' : 'chevron-up',
    s: {
      width: 14,
      height: 14,
      color: 'var(--text-faint)'
    }
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fw-semibold) var(--fs-micro) var(--font-sans)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: 'var(--text-muted)'
    }
  }, "Activity"), /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 5,
      font: '10px var(--font-mono)',
      color: 'var(--success-ink)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      width: 6,
      height: 6,
      borderRadius: 999,
      background: 'var(--success-solid)'
    }
  }), " runtime healthy"), latest && !open && /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 6,
      minWidth: 0,
      font: 'var(--fs-meta) var(--font-mono)',
      color: 'var(--text-muted)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-faint)'
    }
  }, "\xB7"), /*#__PURE__*/React.createElement(Ico, {
    n: DOCK_ICON[latest.kind] || 'dot',
    s: {
      width: 12,
      height: 12,
      color: latest.kind === 'brain' ? 'var(--brain-ink)' : 'var(--text-faint)'
    }
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap',
      maxWidth: 420,
      color: 'var(--text-secondary)'
    }
  }, latest.text), /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-faint)'
    }
  }, latest.t)), /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: 'auto',
      font: '10px var(--font-mono)',
      color: 'var(--text-faint)'
    }
  }, events.length, " events")), open && /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      minHeight: 0,
      display: 'flex',
      flexDirection: 'column'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      padding: '0 12px 6px'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fs-meta) var(--font-mono)',
      color: 'var(--text-faint)'
    }
  }, projectName), /*#__PURE__*/React.createElement("button", {
    onClick: onOpenAudit,
    style: {
      marginLeft: 'auto',
      display: 'inline-flex',
      alignItems: 'center',
      gap: 5,
      padding: '3px 8px',
      borderRadius: 'var(--r-1)',
      border: '1px solid var(--border-default)',
      background: 'transparent',
      cursor: 'pointer',
      font: 'var(--fs-meta) var(--font-sans)',
      color: 'var(--text-secondary)'
    }
  }, /*#__PURE__*/React.createElement(Ico, {
    n: "scroll-text",
    s: {
      width: 13,
      height: 13
    }
  }), " Full audit")), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      overflowY: 'auto',
      padding: '0 12px 10px'
    }
  }, events.length === 0 ? /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fs-meta) var(--font-sans)',
      color: 'var(--text-faint)',
      padding: '8px 2px'
    }
  }, "No recent activity for this project.") : events.map((e, i) => /*#__PURE__*/React.createElement("div", {
    key: i,
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 9,
      padding: '5px 0',
      borderBottom: '1px solid var(--border-subtle)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: e.kind === 'brain' ? 'var(--brain-ink)' : e.result === 'denied' ? 'var(--danger-ink)' : e.result === 'approved' ? 'var(--success-ink)' : 'var(--text-faint)'
    }
  }, /*#__PURE__*/React.createElement(Ico, {
    n: DOCK_ICON[e.kind] || 'dot',
    s: {
      width: 13,
      height: 13
    }
  })), /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1,
      minWidth: 0,
      font: 'var(--fs-meta)/1.4 var(--font-sans)',
      color: 'var(--text-secondary)',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap'
    }
  }, e.text), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fs-micro) var(--font-mono)',
      color: e.actor === 'You' ? 'var(--accent-ink)' : 'var(--text-faint)'
    }
  }, e.actor), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fs-micro) var(--font-mono)',
      color: 'var(--text-faint)',
      width: 56,
      textAlign: 'right'
    }
  }, e.t))))));
}
window.KitShell = {
  TopBar,
  Sidebar,
  Eyebrow,
  Ico,
  EventDock
};
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/control-plane/kit-shell.jsx", error: String((e && e.message) || e) }); }

// ui_kits/control-plane/kit-tasks.jsx
try { (() => {
/* ============================================================
   Control Plane UI kit — Task Inbox drawer + Dispatch dialog.
   Drawer-first task intake: GitHub issues, Linear tickets, plan
   tasks. Drag a chip onto a session/the command center, or click
   to choose how to start (single session vs cc-crew /team-start).
   ============================================================ */
const _NS7 = window.ControlPlaneDesignSystem_a21911;
const {
  Button: TBtn,
  IconButton: TIconBtn,
  StatusPill: TPill,
  RiskBadge: TRisk,
  HarnessBadge: THarness,
  ProfileBadge: TProfile,
  MetaChip: TMeta
} = _NS7;
const TBadge = _NS7.Badge || (({
  children,
  mono,
  style = {}
}) => /*#__PURE__*/React.createElement("span", {
  style: {
    font: `var(--fw-medium) 11px/1 ${mono ? 'var(--font-mono)' : 'var(--font-sans)'}`,
    ...style
  }
}, children));
const {
  Ico: TIco,
  Eyebrow: TEye
} = window.KitShell;
const KD7 = window.KIT;
const {
  useState: useS7
} = React;
const SOURCE = {
  linear: {
    icon: 'square-kanban',
    label: 'Linear',
    tone: 'var(--domain-linear)',
    surf: 'var(--domain-linear-surface)'
  },
  github: {
    icon: 'circle-dot',
    label: 'GitHub',
    tone: 'var(--slate-ink)',
    surf: 'var(--slate-surface)'
  },
  plan: {
    icon: 'list-checks',
    label: 'Plan',
    tone: 'var(--brain-ink)',
    surf: 'var(--brain-surface)'
  }
};
const PRIO = {
  Urgent: 'var(--danger-ink)',
  High: 'var(--attention-ink)',
  Medium: 'var(--caution-ink)',
  Low: 'var(--text-muted)'
};
const TABS = ['All', 'Linear', 'GitHub', 'Plan tasks', 'Completed'];

/* ---------------- Task Inbox drawer ---------------- */
function TaskInbox({
  open,
  onClose,
  onDispatch,
  onDropToast
}) {
  const [tab, setTab] = useS7('All');
  if (!open) return null;
  const tasks = KD7.tasks.filter(t => {
    if (tab === 'All') return t.status !== 'Done';
    if (tab === 'Linear') return t.source === 'linear';
    if (tab === 'GitHub') return t.source === 'github';
    if (tab === 'Plan tasks') return t.source === 'plan';
    if (tab === 'Completed') return t.status === 'Done';
    return true;
  });
  const counts = {
    Linear: KD7.tasks.filter(t => t.source === 'linear').length,
    GitHub: KD7.tasks.filter(t => t.source === 'github').length
  };
  return /*#__PURE__*/React.createElement("div", {
    onClick: onClose,
    style: {
      position: 'fixed',
      inset: 0,
      zIndex: 'var(--z-drawer)',
      display: 'flex',
      justifyContent: 'flex-end',
      background: 'var(--scrim)',
      backdropFilter: 'blur(2px)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    onClick: e => e.stopPropagation(),
    style: {
      width: 400,
      height: '100%',
      background: 'var(--surface-panel)',
      borderLeft: '1px solid var(--border-strong)',
      boxShadow: 'var(--elev-4)',
      display: 'flex',
      flexDirection: 'column',
      animation: 'cp-slide-in 0.24s var(--ease-out)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 9,
      padding: '12px 14px',
      borderBottom: '1px solid var(--border-default)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-secondary)'
    }
  }, /*#__PURE__*/React.createElement(TIco, {
    n: "inbox"
  })), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fw-semibold) var(--fs-sub) var(--font-sans)'
    }
  }, "Task Inbox"), /*#__PURE__*/React.createElement(TBadge, {
    mono: true,
    style: {
      color: 'var(--text-muted)'
    }
  }, KD7.tasks.filter(t => t.status !== 'Done').length, " open"), /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: 'auto',
      display: 'flex',
      gap: 4
    }
  }, /*#__PURE__*/React.createElement(TIconBtn, {
    icon: /*#__PURE__*/React.createElement(TIco, {
      n: "refresh-cw",
      s: {
        width: 15,
        height: 15
      }
    }),
    size: "sm",
    "aria-label": "Sync"
  }), /*#__PURE__*/React.createElement(TIconBtn, {
    icon: /*#__PURE__*/React.createElement(TIco, {
      n: "x",
      s: {
        width: 15,
        height: 15
      }
    }),
    size: "sm",
    "aria-label": "Close",
    onClick: onClose
  }))), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 4,
      padding: '8px 12px',
      borderBottom: '1px solid var(--border-subtle)',
      overflowX: 'auto'
    }
  }, TABS.map(t => /*#__PURE__*/React.createElement("button", {
    key: t,
    onClick: () => setTab(t),
    style: {
      whiteSpace: 'nowrap',
      padding: '4px 9px',
      borderRadius: 999,
      cursor: 'pointer',
      border: `1px solid ${tab === t ? 'var(--accent-line)' : 'var(--border-default)'}`,
      background: tab === t ? 'var(--accent-surface)' : 'transparent',
      color: tab === t ? 'var(--accent-ink)' : 'var(--text-muted)',
      font: 'var(--fw-medium) var(--fs-meta) var(--font-sans)'
    }
  }, t, counts[t] ? ' · ' + counts[t] : ''))), /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '7px 10px 4px',
      font: 'var(--fs-micro) var(--font-sans)',
      color: 'var(--text-faint)',
      display: 'flex',
      alignItems: 'center',
      gap: 6
    }
  }, /*#__PURE__*/React.createElement(TIco, {
    n: "grip-vertical",
    s: {
      width: 12,
      height: 12
    }
  }), " Drag a task onto a session, or click to dispatch"), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      overflowY: 'auto',
      padding: '4px 10px 14px',
      display: 'flex',
      flexDirection: 'column',
      gap: 7
    }
  }, tasks.map(t => /*#__PURE__*/React.createElement(TaskChip, {
    key: t.source + t.id,
    task: t,
    onDispatch: onDispatch
  })), tasks.length === 0 && /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fs-meta) var(--font-sans)',
      color: 'var(--text-faint)',
      padding: '12px 4px'
    }
  }, "Nothing here."))));
}
function TaskChip({
  task,
  onDispatch,
  compact
}) {
  const s = SOURCE[task.source];
  const onDragStart = e => {
    e.dataTransfer.setData('text/plain', task.source + ':' + task.id);
    e.dataTransfer.effectAllowed = 'copy';
    window.__dragTask = task;
  };
  return /*#__PURE__*/React.createElement("div", {
    draggable: true,
    onDragStart: onDragStart,
    onDragEnd: () => {
      window.__dragTask = null;
    },
    onClick: () => onDispatch && onDispatch(task),
    title: "Drag onto a session or click to dispatch",
    style: {
      cursor: 'grab',
      border: '1px solid var(--border-default)',
      borderRadius: 'var(--r-2)',
      background: 'var(--surface-card)',
      padding: '9px 10px',
      display: 'flex',
      flexDirection: 'column',
      gap: 7
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 7
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 4,
      height: 16,
      padding: '0 5px',
      borderRadius: 'var(--r-1)',
      background: s.surf,
      color: s.tone,
      font: 'var(--fw-medium) var(--fs-micro) var(--font-mono)'
    }
  }, /*#__PURE__*/React.createElement(TIco, {
    n: s.icon,
    s: {
      width: 11,
      height: 11
    }
  }), " ", task.id), /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: 'auto',
      display: 'inline-flex',
      alignItems: 'center',
      gap: 4,
      font: 'var(--fs-micro) var(--font-mono)',
      color: PRIO[task.priority]
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      width: 5,
      height: 5,
      borderRadius: 999,
      background: 'currentColor'
    }
  }), task.priority)), /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fw-medium) var(--fs-label)/1.3 var(--font-sans)',
      color: 'var(--text-primary)'
    }
  }, task.title), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 5,
      flexWrap: 'wrap'
    }
  }, task.labels.map(l => /*#__PURE__*/React.createElement("span", {
    key: l,
    style: {
      font: 'var(--fs-micro) var(--font-mono)',
      color: 'var(--text-muted)',
      background: 'var(--neutral-surface)',
      padding: '1px 5px',
      borderRadius: 'var(--r-1)'
    }
  }, l)), task.planTask && /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fs-micro) var(--font-mono)',
      color: 'var(--brain-ink)'
    }
  }, "\u21B3 ", task.planTask), task.session && /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: 'auto',
      font: 'var(--fs-micro) var(--font-mono)',
      color: 'var(--live-ink)'
    }
  }, "\u25CF in session")));
}

/* ---------------- Dispatch dialog ---------------- */
function DispatchDialog({
  task,
  onClose,
  onConfirm
}) {
  const [mode, setMode] = useS7('single');
  const [harness, setHarness] = useS7(task ? task.harness : 'claude-code');
  const [profile, setProfile] = useS7(task ? task.profile : 'Claude Max Main');
  if (!task) return null;
  const s = SOURCE[task.source];
  const profiles = KD7.profilesDetail.map(p => p.name);
  return /*#__PURE__*/React.createElement("div", {
    onClick: onClose,
    style: {
      position: 'fixed',
      inset: 0,
      zIndex: 'var(--z-modal)',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      background: 'var(--scrim)',
      backdropFilter: 'blur(2px)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    onClick: e => e.stopPropagation(),
    style: {
      width: 520,
      background: 'var(--surface-card)',
      border: '1px solid var(--border-strong)',
      borderRadius: 'var(--r-4)',
      boxShadow: 'var(--elev-4)',
      overflow: 'hidden',
      animation: 'cp-pop-in 0.18s var(--ease-out)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 9,
      padding: '13px 16px',
      borderBottom: '1px solid var(--border-default)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-secondary)'
    }
  }, /*#__PURE__*/React.createElement(TIco, {
    n: "rocket"
  })), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fw-semibold) var(--fs-sub) var(--font-sans)'
    }
  }, "Dispatch task"), /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: 'auto',
      display: 'inline-flex',
      alignItems: 'center',
      gap: 5,
      height: 18,
      padding: '0 7px',
      borderRadius: 'var(--r-1)',
      background: s.surf,
      color: s.tone,
      font: 'var(--fw-medium) var(--fs-micro) var(--font-mono)'
    }
  }, /*#__PURE__*/React.createElement(TIco, {
    n: s.icon,
    s: {
      width: 11,
      height: 11
    }
  }), " ", task.id)), /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '16px'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fw-medium) var(--fs-body-lg)/1.35 var(--font-sans)',
      marginBottom: 4
    }
  }, task.title), /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fs-meta) var(--font-mono)',
      color: 'var(--text-muted)',
      marginBottom: 14
    }
  }, task.planTask ? '↳ ' + task.planTask + ' · ' : '', "suggested ", task.branch || 'new worktree'), /*#__PURE__*/React.createElement(TEye, {
    style: {
      marginBottom: 8
    }
  }, "How to start"), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 7,
      marginBottom: 14
    }
  }, /*#__PURE__*/React.createElement(ModeOpt, {
    active: mode === 'single',
    onClick: () => setMode('single'),
    icon: "bot",
    title: "Single session",
    desc: "One agent in a fresh worktree."
  }), /*#__PURE__*/React.createElement(ModeOpt, {
    active: mode === 'team',
    onClick: () => setMode('team'),
    icon: "users-round",
    title: "Agent team \u2014 cc-crew /team-start",
    desc: "Orchestrator decomposes and delegates to workers.",
    tone: "teal"
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      gridTemplateColumns: mode === 'single' ? '1fr 1fr' : '1fr',
      gap: 10
    }
  }, mode === 'single' && /*#__PURE__*/React.createElement(Field, {
    label: "Harness"
  }, /*#__PURE__*/React.createElement("select", {
    value: harness,
    onChange: e => setHarness(e.target.value),
    style: selStyle
  }, /*#__PURE__*/React.createElement("option", {
    value: "claude-code"
  }, "Claude Code"), /*#__PURE__*/React.createElement("option", {
    value: "codex-cli"
  }, "Codex CLI"), /*#__PURE__*/React.createElement("option", {
    value: "codex-cloud"
  }, "Codex Cloud"))), /*#__PURE__*/React.createElement(Field, {
    label: "Execution profile"
  }, /*#__PURE__*/React.createElement("select", {
    value: profile,
    onChange: e => setProfile(e.target.value),
    style: selStyle
  }, profiles.map(p => /*#__PURE__*/React.createElement("option", {
    key: p
  }, p))))), /*#__PURE__*/React.createElement("div", {
    style: {
      marginTop: 14,
      background: 'var(--surface-sunken)',
      borderRadius: 'var(--r-2)',
      padding: '10px 12px',
      boxShadow: 'var(--elev-inset)'
    }
  }, /*#__PURE__*/React.createElement(TEye, {
    style: {
      marginBottom: 7
    }
  }, "Will run via Gateway"), /*#__PURE__*/React.createElement("code", {
    style: {
      font: 'var(--fs-meta) var(--font-mono)',
      color: 'var(--accent-ink)'
    }
  }, mode === 'team' ? `/team-start ${task.branch ? task.branch.split('/')[0] : 'task'} --profile "${profile}"` : `start ${harness} --profile "${profile}" --task ${task.id}`))), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 8,
      padding: '12px 16px',
      borderTop: '1px solid var(--border-default)'
    }
  }, /*#__PURE__*/React.createElement(TBtn, {
    variant: "ghost",
    size: "md",
    onClick: onClose,
    style: {
      marginRight: 'auto'
    }
  }, "Cancel"), /*#__PURE__*/React.createElement(TBtn, {
    variant: "secondary",
    size: "md",
    icon: /*#__PURE__*/React.createElement(TIco, {
      n: "link",
      s: {
        width: 14,
        height: 14
      }
    })
  }, "Link to plan"), /*#__PURE__*/React.createElement(TBtn, {
    variant: "primary",
    size: "md",
    icon: /*#__PURE__*/React.createElement(TIco, {
      n: mode === 'team' ? 'users-round' : 'play',
      s: {
        width: 14,
        height: 14
      }
    }),
    onClick: () => onConfirm(task, {
      mode,
      harness,
      profile
    })
  }, mode === 'team' ? 'Start agent team' : 'Dispatch session'))));
}
const selStyle = {
  width: '100%',
  background: 'var(--surface-input)',
  color: 'var(--text-primary)',
  border: '1px solid var(--border-default)',
  borderRadius: 'var(--r-2)',
  font: 'var(--fs-label) var(--font-sans)',
  padding: '6px 8px',
  height: 'var(--ctl-md)'
};
function Field({
  label,
  children
}) {
  return /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fs-meta) var(--font-sans)',
      color: 'var(--text-muted)',
      marginBottom: 5
    }
  }, label), children);
}
function ModeOpt({
  active,
  onClick,
  icon,
  title,
  desc,
  tone
}) {
  const ac = tone === 'teal' ? 'var(--teal-line)' : 'var(--accent-line)';
  const sf = tone === 'teal' ? 'var(--teal-surface)' : 'var(--accent-surface)';
  const ink = tone === 'teal' ? 'var(--teal-ink)' : 'var(--accent-ink)';
  return /*#__PURE__*/React.createElement("button", {
    onClick: onClick,
    style: {
      textAlign: 'left',
      cursor: 'pointer',
      display: 'flex',
      alignItems: 'center',
      gap: 11,
      padding: '10px 12px',
      borderRadius: 'var(--r-2)',
      border: `1px solid ${active ? ac : 'var(--border-default)'}`,
      background: active ? sf : 'var(--surface-card)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      width: 30,
      height: 30,
      flex: 'none',
      borderRadius: 'var(--r-2)',
      background: active ? sf : 'var(--surface-active)',
      color: active ? ink : 'var(--text-muted)',
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center'
    }
  }, /*#__PURE__*/React.createElement(TIco, {
    n: icon,
    s: {
      width: 16,
      height: 16
    }
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fw-medium) var(--fs-body) var(--font-sans)',
      color: active ? ink : 'var(--text-primary)'
    }
  }, title), /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fs-meta) var(--font-sans)',
      color: 'var(--text-muted)',
      marginTop: 2
    }
  }, desc)), /*#__PURE__*/React.createElement("span", {
    style: {
      width: 16,
      height: 16,
      flex: 'none',
      borderRadius: 999,
      border: `2px solid ${active ? ink : 'var(--border-strong)'}`,
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center'
    }
  }, active && /*#__PURE__*/React.createElement("span", {
    style: {
      width: 7,
      height: 7,
      borderRadius: 999,
      background: ink
    }
  })));
}
window.KitTasks = {
  TaskInbox,
  DispatchDialog,
  TaskChip,
  HumanInputQueue
};

/* ---------------- Human Input Queue (Screen 15) ---------------- */
const HIQ_RISK_ICON = {
  gateway: 'shield-check',
  decision: 'git-pull-request',
  personalization: 'wand-sparkles'
};
function HumanInputQueue({
  open,
  onClose,
  onResolve,
  resolved = []
}) {
  if (!open) return null;
  const items = KD7.humanInput.filter(it => !resolved.includes(it.id));
  const groups = {};
  items.forEach(it => {
    (groups[it.group] = groups[it.group] || []).push(it);
  });
  const order = ['Permission requests', 'High-risk actions', 'Failed checks · needs decision', 'Workflow personalization'];
  return /*#__PURE__*/React.createElement("div", {
    onClick: onClose,
    style: {
      position: 'fixed',
      inset: 0,
      zIndex: 'var(--z-drawer)',
      display: 'flex',
      justifyContent: 'flex-end',
      background: 'var(--scrim)',
      backdropFilter: 'blur(2px)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    onClick: e => e.stopPropagation(),
    style: {
      width: 420,
      height: '100%',
      background: 'var(--surface-panel)',
      borderLeft: '1px solid var(--attention-line)',
      boxShadow: 'var(--elev-4)',
      display: 'flex',
      flexDirection: 'column',
      animation: 'cp-slide-in 0.24s var(--ease-out)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 9,
      padding: '12px 14px',
      borderBottom: '1px solid var(--border-default)',
      background: 'var(--attention-surface)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--attention-ink)'
    }
  }, /*#__PURE__*/React.createElement(TIco, {
    n: "circle-alert"
  })), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fw-semibold) var(--fs-sub) var(--font-sans)',
      color: 'var(--text-primary)'
    }
  }, "Human input"), /*#__PURE__*/React.createElement(TBadge, {
    mono: true,
    style: {
      color: 'var(--attention-ink)'
    }
  }, items.length, " waiting"), /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: 'auto'
    }
  }, /*#__PURE__*/React.createElement(TIconBtn, {
    icon: /*#__PURE__*/React.createElement(TIco, {
      n: "x",
      s: {
        width: 15,
        height: 15
      }
    }),
    "aria-label": "Close",
    onClick: onClose
  }))), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      overflowY: 'auto',
      padding: '12px'
    }
  }, items.length === 0 && /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      gap: 8,
      padding: '40px 16px',
      color: 'var(--text-muted)'
    }
  }, /*#__PURE__*/React.createElement(TIco, {
    n: "check-check",
    s: {
      width: 26,
      height: 26,
      color: 'var(--success-ink)'
    }
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fs-label) var(--font-sans)'
    }
  }, "Queue clear \u2014 nothing waiting on you.")), order.filter(g => groups[g]).map(g => /*#__PURE__*/React.createElement("div", {
    key: g,
    style: {
      marginBottom: 14
    }
  }, /*#__PURE__*/React.createElement(TEye, {
    style: {
      marginBottom: 8
    }
  }, g), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 8
    }
  }, groups[g].map(it => /*#__PURE__*/React.createElement(HIQCard, {
    key: it.id,
    it: it,
    onResolve: onResolve
  }))))))));
}
function HIQCard({
  it,
  onResolve
}) {
  const loud = it.risk === 'high' || it.risk === 'critical';
  return /*#__PURE__*/React.createElement("div", {
    style: {
      border: `1px solid ${loud ? 'var(--danger-line)' : 'var(--border-default)'}`,
      background: loud ? 'var(--danger-surface)' : 'var(--surface-card)',
      borderRadius: 'var(--r-3)',
      padding: '11px 12px'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 7,
      marginBottom: 8
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-muted)'
    }
  }, /*#__PURE__*/React.createElement(TIco, {
    n: HIQ_RISK_ICON[it.kind] || 'circle-alert',
    s: {
      width: 14,
      height: 14
    }
  })), /*#__PURE__*/React.createElement(TRisk, {
    level: it.risk
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: 'auto',
      font: 'var(--fs-micro) var(--font-mono)',
      color: 'var(--text-faint)'
    }
  }, it.age)), /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fw-medium) var(--fs-label)/1.4 var(--font-sans)',
      color: 'var(--text-primary)',
      marginBottom: 5
    }
  }, it.reason), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 6,
      font: 'var(--fs-micro) var(--font-mono)',
      color: 'var(--text-faint)',
      marginBottom: 11
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--accent-ink)'
    }
  }, it.actor), /*#__PURE__*/React.createElement("span", null, "\u2192"), /*#__PURE__*/React.createElement("span", null, it.target)), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 7
    }
  }, /*#__PURE__*/React.createElement(TBtn, {
    variant: loud ? 'danger' : 'attention',
    size: "sm",
    icon: /*#__PURE__*/React.createElement(TIco, {
      n: it.kind === 'personalization' ? 'wand-sparkles' : 'check',
      s: {
        width: 13,
        height: 13
      }
    }),
    onClick: () => onResolve(it, 'open')
  }, it.kind === 'gateway' ? 'Review' : it.kind === 'decision' ? 'Decide' : 'Personalize'), it.kind === 'gateway' && /*#__PURE__*/React.createElement(TBtn, {
    variant: "ghost",
    size: "sm",
    onClick: () => onResolve(it, 'deny')
  }, "Deny"), it.kind !== 'gateway' && /*#__PURE__*/React.createElement(TBtn, {
    variant: "ghost",
    size: "sm",
    onClick: () => onResolve(it, 'defer')
  }, "Defer")));
}
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/control-plane/kit-tasks.jsx", error: String((e && e.message) || e) }); }

// ui_kits/control-plane/kit-views.jsx
try { (() => {
/* ============================================================
   Control Plane UI kit — main views.
   CommandCenter · ProjectGraph · SessionTerminal · DiffReview
   + SessionInspector. Composes DS components from the window NS.
   ============================================================ */
const _NS = window.ControlPlaneDesignSystem_a21911;
const {
  Button: VBtn,
  IconButton: VIconBtn,
  StatusPill: VPill,
  RiskBadge: VRisk,
  UsageMeter: VMeter,
  AttentionMarker: VMark,
  HarnessBadge: VHarness,
  ProfileBadge: VProfile,
  MetaChip: VMeta,
  SessionRow: VSessionRow,
  GraphNode: VNode,
  EvidenceChip: VEvidence,
  DiffHunk: VDiff
} = _NS;
const VBadge = _NS.Badge || (({
  children,
  mono,
  tone,
  style = {}
}) => /*#__PURE__*/React.createElement("span", {
  style: {
    display: 'inline-flex',
    alignItems: 'center',
    gap: 4,
    height: 18,
    padding: '0 6px',
    borderRadius: 'var(--r-1)',
    background: 'var(--neutral-surface)',
    color: 'var(--text-secondary)',
    font: `var(--fw-medium) 11px/1 ${mono ? 'var(--font-mono)' : 'var(--font-sans)'}`,
    ...style
  }
}, children));
const {
  Ico: VIco,
  Eyebrow: VEye
} = window.KitShell;
const KD = window.KIT;
const panel = {
  background: 'var(--surface-canvas)',
  overflow: 'hidden'
};
const sectionPad = {
  padding: '14px 16px'
};

/* ---------------- Command Center ---------------- */
function CommandCenter({
  setSel,
  setView,
  openGateway,
  queue,
  overrides = {},
  project = 'cp',
  onTaskDrop
}) {
  const proj = KD.projects.find(p => p.id === project) || {
    id: 'all',
    name: 'All projects'
  };
  const projSessions = project === 'all' ? KD.sessions : KD.sessions.filter(s => s.proj === project);
  const sessions = projSessions.map(s => overrides[s.id] ? {
    ...s,
    status: overrides[s.id]
  } : s);
  const attention = sessions.filter(s => ['waiting-human', 'failed'].includes(s.status));
  const working = sessions.filter(s => ['running', 'active'].includes(s.status));
  const done = sessions.filter(s => ['completed', 'idle', 'archived'].includes(s.status));
  const open = id => {
    const s = projSessions.find(x => x.id === id);
    setSel(id);
    setView(s && s.team ? 'team' : 'terminal');
  };
  const [dragOver, setDragOver] = React.useState(false);
  const onColDrop = e => {
    e.preventDefault();
    setDragOver(false);
    const t = window.__dragTask;
    if (t && onTaskDrop) onTaskDrop(t, null);
  };
  return /*#__PURE__*/React.createElement("div", {
    style: {
      ...panel,
      display: 'grid',
      gridTemplateColumns: '1fr 300px',
      height: '100%'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      overflowY: 'auto',
      position: 'relative'
    },
    onDragOver: e => {
      if (window.__dragTask) {
        e.preventDefault();
        setDragOver(true);
      }
    },
    onDragLeave: e => {
      if (e.currentTarget === e.target) setDragOver(false);
    },
    onDrop: onColDrop
  }, dragOver && /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      inset: 6,
      zIndex: 20,
      pointerEvents: 'none',
      border: '2px dashed var(--accent-line)',
      borderRadius: 'var(--r-3)',
      background: 'var(--accent-surface)',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      font: 'var(--fw-medium) var(--fs-body) var(--font-sans)',
      color: 'var(--accent-ink)'
    }
  }, "Drop to dispatch a new session"), /*#__PURE__*/React.createElement("div", {
    style: {
      ...sectionPad,
      display: 'flex',
      alignItems: 'center',
      gap: 10,
      borderBottom: '1px solid var(--border-subtle)'
    }
  }, /*#__PURE__*/React.createElement("button", {
    onClick: () => setView('projects'),
    title: "All projects",
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 6,
      padding: '3px 8px 3px 6px',
      borderRadius: 'var(--r-1)',
      border: '1px solid var(--border-default)',
      background: 'var(--surface-card)',
      cursor: 'pointer',
      font: 'var(--fs-meta) var(--font-sans)',
      color: 'var(--text-muted)'
    }
  }, /*#__PURE__*/React.createElement(VIco, {
    n: "chevron-left",
    s: {
      width: 13,
      height: 13
    }
  }), " Projects"), /*#__PURE__*/React.createElement("h1", {
    style: {
      margin: 0,
      font: 'var(--fw-semibold) var(--fs-h2)/1.1 var(--font-sans)',
      letterSpacing: 'var(--tracking-tight)'
    }
  }, proj.name), /*#__PURE__*/React.createElement(VBadge, {
    tone: "neutral",
    mono: true
  }, sessions.length, " sessions"), /*#__PURE__*/React.createElement("div", {
    style: {
      marginLeft: 'auto',
      display: 'flex',
      gap: 7
    }
  }, /*#__PURE__*/React.createElement(VBtn, {
    variant: "ghost",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(VIco, {
      n: "arrow-up-down"
    })
  }, "Sort: attention"), /*#__PURE__*/React.createElement(VBtn, {
    variant: "primary",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(VIco, {
      n: "plus"
    })
  }, "New session"))), /*#__PURE__*/React.createElement("div", {
    style: sectionPad
  }, /*#__PURE__*/React.createElement(VEye, {
    style: {
      marginBottom: 10,
      color: attention.length ? 'var(--attention-ink)' : 'var(--text-faint)'
    }
  }, "\u25CF Needs my attention"), attention.length === 0 ? /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fs-label) var(--font-sans)',
      color: 'var(--text-muted)',
      display: 'flex',
      alignItems: 'center',
      gap: 7,
      padding: '6px 0'
    }
  }, /*#__PURE__*/React.createElement(VIco, {
    n: "check-check",
    s: {
      width: 15,
      height: 15,
      color: 'var(--success-ink)'
    }
  }), " All clear. No session is waiting on you.") : /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      gridTemplateColumns: '1fr 1fr',
      gap: 10
    }
  }, attention.map(s => /*#__PURE__*/React.createElement(AttentionCard, {
    key: s.id,
    s: s,
    onOpen: () => open(s.id),
    openGateway: openGateway
  })))), /*#__PURE__*/React.createElement("div", {
    style: sectionPad
  }, /*#__PURE__*/React.createElement(VEye, {
    style: {
      marginBottom: 8
    }
  }, "\u25B6 Working now"), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 6
    }
  }, working.map(s => /*#__PURE__*/React.createElement(SessionRowWrap, {
    key: s.id,
    s: s,
    onOpen: () => open(s.id),
    onTaskDrop: onTaskDrop
  })))), /*#__PURE__*/React.createElement("div", {
    style: sectionPad
  }, /*#__PURE__*/React.createElement(VEye, {
    style: {
      marginBottom: 8
    }
  }, "Recently settled"), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 6,
      opacity: 0.85
    }
  }, done.map(s => /*#__PURE__*/React.createElement(SessionRowWrap, {
    key: s.id,
    s: s,
    onOpen: () => open(s.id),
    onTaskDrop: onTaskDrop
  }))))), /*#__PURE__*/React.createElement(CommandRail, {
    openGateway: openGateway,
    queue: queue
  }));
}
function AttentionCard({
  s,
  onOpen,
  openGateway
}) {
  const waiting = s.status === 'waiting-human';
  return /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'relative',
      border: `1px solid ${waiting ? 'var(--attention-line)' : 'var(--danger-line)'}`,
      background: waiting ? 'var(--attention-surface)' : 'var(--danger-surface)',
      borderRadius: 'var(--r-3)',
      padding: '11px 12px 12px',
      overflow: 'hidden'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      position: 'absolute',
      left: 0,
      top: 0,
      bottom: 0,
      width: 3,
      background: waiting ? 'var(--attention-solid)' : 'var(--danger-solid)'
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 7,
      marginBottom: 8
    }
  }, /*#__PURE__*/React.createElement(VPill, {
    status: s.status,
    beacon: waiting
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: 'auto'
    }
  }, /*#__PURE__*/React.createElement(VHarness, {
    harness: s.harness,
    showLabel: false
  }))), /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fw-medium) var(--fs-body)/1.3 var(--font-sans)',
      marginBottom: 6
    }
  }, s.title), /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fs-meta) var(--font-mono)',
      color: 'var(--text-muted)',
      marginBottom: 11,
      whiteSpace: 'nowrap',
      overflow: 'hidden',
      textOverflow: 'ellipsis'
    }
  }, s.current), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 6
    }
  }, waiting ? /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(VBtn, {
    variant: "attention",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(VIco, {
      n: "check"
    }),
    onClick: () => openGateway('q1')
  }, "Review & approve"), /*#__PURE__*/React.createElement(VBtn, {
    variant: "ghost",
    size: "sm",
    onClick: onOpen
  }, "Open")) : /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(VBtn, {
    variant: "secondary",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(VIco, {
      n: "refresh-cw"
    })
  }, "Retry checks"), /*#__PURE__*/React.createElement(VBtn, {
    variant: "ghost",
    size: "sm",
    onClick: onOpen
  }, "Open log"))));
}
function SessionRowWrap({
  s,
  onOpen,
  onTaskDrop
}) {
  const [over, setOver] = React.useState(false);
  return /*#__PURE__*/React.createElement("div", {
    onDragOver: e => {
      if (window.__dragTask) {
        e.preventDefault();
        e.stopPropagation();
        setOver(true);
      }
    },
    onDragLeave: () => setOver(false),
    onDrop: e => {
      e.preventDefault();
      e.stopPropagation();
      setOver(false);
      const t = window.__dragTask;
      if (t && onTaskDrop) onTaskDrop(t, s);
    },
    style: {
      borderRadius: 'var(--r-2)',
      boxShadow: over ? 'inset 0 0 0 2px var(--accent-line)' : 'none'
    }
  }, /*#__PURE__*/React.createElement(VSessionRow, {
    status: s.status,
    title: s.title,
    harness: s.harness,
    profile: s.profile,
    task: s.task,
    branch: s.branch,
    worktree: s.worktree,
    pr: s.pr,
    context: s.context,
    current: s.current,
    activity: s.activity,
    team: s.team,
    onClick: onOpen
  }));
}
function CommandRail({
  openGateway,
  queue = []
}) {
  return /*#__PURE__*/React.createElement("aside", {
    style: {
      borderLeft: '1px solid var(--border-default)',
      background: 'var(--surface-panel)',
      overflowY: 'auto',
      display: 'flex',
      flexDirection: 'column'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      ...sectionPad,
      borderBottom: '1px solid var(--border-subtle)'
    }
  }, /*#__PURE__*/React.createElement(VEye, {
    style: {
      marginBottom: 10
    }
  }, "Human input queue ", queue.length > 0 && /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--attention-ink)'
    }
  }, "\xB7 ", queue.length)), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 8
    }
  }, queue.length === 0 ? /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fs-meta) var(--font-sans)',
      color: 'var(--text-faint)',
      padding: '6px 2px',
      display: 'flex',
      alignItems: 'center',
      gap: 6
    }
  }, /*#__PURE__*/React.createElement(VIco, {
    n: "check-check",
    s: {
      width: 14,
      height: 14,
      color: 'var(--success-ink)'
    }
  }), " Queue clear \u2014 nothing waiting on you.") : queue.map(q => /*#__PURE__*/React.createElement(QueueItem, {
    key: q.id,
    risk: q.risk,
    who: q.who,
    text: q.short,
    onClick: () => openGateway(q.id)
  })))), /*#__PURE__*/React.createElement("div", {
    style: {
      ...sectionPad,
      borderBottom: '1px solid var(--border-subtle)'
    }
  }, /*#__PURE__*/React.createElement(VEye, {
    style: {
      marginBottom: 10
    }
  }, "Capacity"), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 10
    }
  }, /*#__PURE__*/React.createElement(VMeter, {
    label: "Aggregate context",
    value: 512,
    max: 1000,
    valueText: "512k / 1M"
  }), /*#__PURE__*/React.createElement(VMeter, {
    label: "Spend today",
    value: 34,
    max: 50,
    valueText: "$34 / $50",
    accuracy: "estimated"
  }), /*#__PURE__*/React.createElement(VMeter, {
    label: "Active runtime",
    value: 3,
    max: 6,
    valueText: "3 / 6 agents"
  }))), /*#__PURE__*/React.createElement("div", {
    style: {
      ...sectionPad
    }
  }, /*#__PURE__*/React.createElement(VEye, {
    style: {
      marginBottom: 10
    }
  }, "Recent events"), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 2
    }
  }, KD.events.slice(0, 6).map((e, i) => /*#__PURE__*/React.createElement(EventLine, {
    key: i,
    e: e
  })))));
}
function QueueItem({
  risk,
  who,
  text,
  onClick
}) {
  return /*#__PURE__*/React.createElement("button", {
    onClick: onClick,
    style: {
      textAlign: 'left',
      cursor: 'pointer',
      border: '1px solid var(--attention-line)',
      background: 'var(--attention-surface)',
      borderRadius: 'var(--r-2)',
      padding: '9px 10px',
      display: 'flex',
      flexDirection: 'column',
      gap: 7
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 6
    }
  }, /*#__PURE__*/React.createElement(VRisk, {
    level: risk
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: 'auto',
      font: 'var(--fs-micro) var(--font-mono)',
      color: 'var(--text-muted)'
    }
  }, who)), /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fw-medium) var(--fs-label)/1.35 var(--font-sans)',
      color: 'var(--text-primary)'
    }
  }, text));
}
const EVENT_ICON = {
  approval: 'shield-question',
  git: 'git-commit-horizontal',
  brain: 'brain',
  pr: 'git-pull-request',
  workflow: 'workflow',
  session: 'circle-check'
};
function EventLine({
  e
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 8,
      padding: '5px 0',
      alignItems: 'flex-start'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      marginTop: 1,
      color: e.kind === 'brain' ? 'var(--brain-ink)' : e.kind === 'approval' ? 'var(--attention-ink)' : 'var(--text-faint)'
    }
  }, /*#__PURE__*/React.createElement(VIco, {
    n: EVENT_ICON[e.kind] || 'dot',
    s: {
      width: 13,
      height: 13
    }
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      minWidth: 0
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fs-meta)/1.4 var(--font-sans)',
      color: 'var(--text-secondary)'
    }
  }, e.text), /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fs-micro) var(--font-mono)',
      color: 'var(--text-faint)',
      marginTop: 1
    }
  }, e.t, " \xB7 ", e.actor)));
}

/* ---------------- Projects overview (top-layer) ---------------- */
const WF_TONE = {
  active: ['Active', 'var(--teal-ink)', 'var(--teal-surface)', 'var(--teal-line)'],
  drift: ['Drift', 'var(--warning-ink)', 'var(--warning-surface)', 'var(--warning-line)'],
  'needs-personalization': ['Needs setup', 'var(--caution-ink)', 'var(--caution-surface)', 'var(--caution-line)'],
  none: ['No pack', 'var(--text-faint)', 'var(--neutral-surface)', 'var(--border-default)']
};
const BRAIN_TONE = {
  ready: ['Brain ready', 'var(--brain-ink)'],
  indexing: ['Indexing', 'var(--live-ink)'],
  stale: ['Brain stale', 'var(--warning-ink)']
};
function ProjectsOverview({
  onSelectProject,
  active
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      height: '100%',
      overflowY: 'auto',
      background: 'var(--surface-canvas)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '16px',
      borderBottom: '1px solid var(--border-subtle)',
      display: 'flex',
      alignItems: 'center',
      gap: 10
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-secondary)'
    }
  }, /*#__PURE__*/React.createElement(VIco, {
    n: "layout-grid"
  })), /*#__PURE__*/React.createElement("h1", {
    style: {
      margin: 0,
      font: 'var(--fw-semibold) var(--fs-h2)/1 var(--font-sans)',
      letterSpacing: 'var(--tracking-tight)'
    }
  }, "Projects"), /*#__PURE__*/React.createElement(VBadge, {
    tone: "neutral",
    mono: true
  }, KD.projects.length), /*#__PURE__*/React.createElement("div", {
    style: {
      marginLeft: 'auto'
    }
  }, /*#__PURE__*/React.createElement(VBtn, {
    variant: "primary",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(VIco, {
      n: "plus"
    })
  }, "Add project"))), /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '16px',
      display: 'grid',
      gridTemplateColumns: 'repeat(auto-fill, minmax(300px, 1fr))',
      gap: 12
    }
  }, KD.projects.map(p => {
    const wf = WF_TONE[p.workflow] || WF_TONE.none;
    const br = BRAIN_TONE[p.brain] || BRAIN_TONE.ready;
    return /*#__PURE__*/React.createElement("button", {
      key: p.id,
      onClick: () => onSelectProject(p.id),
      style: {
        textAlign: 'left',
        cursor: 'pointer',
        border: `1px solid ${active === p.id ? 'var(--accent-line)' : 'var(--border-default)'}`,
        borderRadius: 'var(--r-3)',
        background: 'var(--surface-card)',
        padding: '14px 15px',
        display: 'flex',
        flexDirection: 'column',
        gap: 11,
        boxShadow: active === p.id ? 'inset 0 0 0 1px var(--accent-line)' : 'none'
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'flex-start',
        gap: 9
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        width: 30,
        height: 30,
        flex: 'none',
        borderRadius: 'var(--r-2)',
        background: 'var(--surface-active)',
        color: 'var(--text-secondary)',
        display: 'inline-flex',
        alignItems: 'center',
        justifyContent: 'center'
      }
    }, /*#__PURE__*/React.createElement(VIco, {
      n: "folder-git-2",
      s: {
        width: 16,
        height: 16
      }
    })), /*#__PURE__*/React.createElement("div", {
      style: {
        minWidth: 0,
        flex: 1
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        font: 'var(--fw-semibold) var(--fs-body-lg)/1.25 var(--font-sans)',
        color: 'var(--text-primary)'
      }
    }, p.name), /*#__PURE__*/React.createElement("div", {
      style: {
        font: 'var(--fs-meta) var(--font-mono)',
        color: 'var(--text-faint)',
        marginTop: 2
      }
    }, p.repo)), p.waiting > 0 && /*#__PURE__*/React.createElement("span", {
      title: "waiting on human",
      style: {
        display: 'inline-flex',
        alignItems: 'center',
        gap: 4,
        height: 18,
        padding: '0 6px',
        borderRadius: 999,
        background: 'var(--attention-solid)',
        color: 'var(--attention-on-solid)',
        font: 'var(--fw-semibold) var(--fs-micro) var(--font-mono)'
      }
    }, "\u25C6 ", p.waiting)), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 14,
        font: 'var(--fs-meta) var(--font-mono)',
        color: 'var(--text-muted)'
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        display: 'inline-flex',
        alignItems: 'center',
        gap: 5
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        width: 6,
        height: 6,
        borderRadius: 999,
        background: 'var(--live-solid)'
      }
    }), p.active, " active"), /*#__PURE__*/React.createElement("span", {
      style: {
        display: 'inline-flex',
        alignItems: 'center',
        gap: 5
      }
    }, /*#__PURE__*/React.createElement(VIco, {
      n: "git-pull-request",
      s: {
        width: 12,
        height: 12
      }
    }), p.prs, " PR"), /*#__PURE__*/React.createElement("span", {
      style: {
        marginLeft: 'auto',
        color: br[1]
      }
    }, br[0])), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 8
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        display: 'inline-flex',
        alignItems: 'center',
        gap: 5,
        height: 20,
        padding: '0 8px',
        borderRadius: 'var(--r-1)',
        background: wf[2],
        border: `1px solid ${wf[3]}`,
        color: wf[1],
        font: 'var(--fw-medium) var(--fs-micro) var(--font-sans)'
      }
    }, /*#__PURE__*/React.createElement(VIco, {
      n: "package",
      s: {
        width: 12,
        height: 12
      }
    }), " ", wf[0]), /*#__PURE__*/React.createElement("span", {
      style: {
        marginLeft: 'auto',
        font: 'var(--fs-meta) var(--font-sans)',
        color: 'var(--accent-ink)',
        display: 'inline-flex',
        alignItems: 'center',
        gap: 3
      }
    }, "Open ", /*#__PURE__*/React.createElement(VIco, {
      n: "arrow-right",
      s: {
        width: 13,
        height: 13
      }
    }))));
  })));
}
window.KitViews = {
  CommandCenter,
  ProjectsOverview
};
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/control-plane/kit-views.jsx", error: String((e && e.message) || e) }); }

// ui_kits/control-plane/kit-views2.jsx
try { (() => {
/* ============================================================
   Control Plane UI kit — graph, terminal & diff views + inspector.
   ============================================================ */
const _NS2 = window.ControlPlaneDesignSystem_a21911;
const {
  Button: GBtn,
  IconButton: GIconBtn,
  StatusPill: GPill,
  RiskBadge: GRisk,
  UsageMeter: GMeter,
  HarnessBadge: GHarness,
  ProfileBadge: GProfile,
  MetaChip: GMeta,
  GraphNode: GNode,
  EvidenceChip: GEvidence,
  DiffHunk: GDiff
} = _NS2;
const GBadge = _NS2.Badge || (({
  children,
  mono,
  tone,
  size,
  variant,
  style = {}
}) => /*#__PURE__*/React.createElement("span", {
  style: {
    display: 'inline-flex',
    alignItems: 'center',
    gap: 4,
    height: 18,
    padding: '0 6px',
    borderRadius: 'var(--r-1)',
    background: 'var(--neutral-surface)',
    color: 'var(--text-secondary)',
    font: `var(--fw-medium) 11px/1 ${mono ? 'var(--font-mono)' : 'var(--font-sans)'}`,
    ...style
  }
}, children));
const {
  Ico: GIco,
  Eyebrow: GEye
} = window.KitShell;
const KD2 = window.KIT;

/* ---------------- Project Graph ---------------- */
const NODES = [{
  id: 'proj',
  proj: 'cp',
  kind: 'project',
  title: 'Control Plane',
  subtitle: 'org/control-plane',
  status: 'active',
  x: 40,
  y: 150,
  owner: null,
  detail: {
    Repo: 'org/control-plane',
    Sessions: '3 active',
    PRs: '2 open',
    Workflow: 'cc-crew · active'
  },
  open: {
    view: 'command',
    label: 'Open project'
  }
}, {
  id: 's1',
  proj: 'cp',
  kind: 'session',
  title: 'ENG-221 · OAuth',
  subtitle: 'claude-code · max-main',
  status: 'waiting-human',
  beacon: true,
  x: 250,
  y: 60,
  meta: ['93% ctx'],
  detail: {
    Harness: 'Claude Code',
    Profile: 'Claude Max Main',
    Branch: 'agent/eng-221-oauth',
    Context: '186k / 200k',
    Activity: '2m ago'
  },
  open: {
    view: 'terminal',
    sel: 's1',
    label: 'Open terminal'
  }
}, {
  id: 's2',
  proj: 'cp',
  kind: 'session',
  title: 'GH-184 · leak',
  subtitle: 'codex-cli',
  status: 'running',
  x: 250,
  y: 230,
  meta: ['48% ctx'],
  detail: {
    Harness: 'Codex CLI',
    Profile: 'Codex CLI Main',
    Branch: 'fix/gh-184-leak',
    Context: '96k / 200k',
    Activity: 'just now'
  },
  open: {
    view: 'terminal',
    sel: 's2',
    label: 'Open terminal'
  }
}, {
  id: 'team',
  proj: 'cp',
  kind: 'team',
  title: 'Phase 2 team',
  subtitle: '1 lead · 3 workers',
  status: 'active',
  x: 250,
  y: 380,
  detail: {
    Pack: 'cc-crew',
    Lead: 'Orchestrator',
    Workers: '3',
    Waiting: '1 · permission',
    Branch: 'team/phase-2-graph'
  },
  open: {
    view: 'terminal',
    team: true,
    label: 'Open all team terminals'
  }
}, {
  id: 'wt1',
  proj: 'cp',
  kind: 'worktree',
  title: 'agent/eng-221',
  status: 'active',
  x: 480,
  y: 60,
  meta: ['+412 −98'],
  detail: {
    Path: '~/wt/eng-221',
    Branch: 'agent/eng-221-oauth',
    Diff: '+412 −98',
    Dirty: 'yes'
  },
  open: {
    view: 'editor',
    label: 'Open in editor'
  }
}, {
  id: 'pr1',
  proj: 'cp',
  kind: 'pr',
  title: '#84 registry',
  subtitle: 'Codex',
  status: 'pr-open',
  x: 690,
  y: 60,
  detail: {
    Number: '#84',
    Author: 'Codex · PR-fix',
    Checks: 'failing',
    Diff: '+412 −98'
  },
  open: {
    view: 'code',
    label: 'Open review'
  }
}, {
  id: 'wt2',
  proj: 'cp',
  kind: 'worktree',
  title: 'fix/gh-184',
  status: 'active',
  x: 480,
  y: 230,
  meta: ['+96 −12'],
  detail: {
    Path: '~/wt/gh-184',
    Branch: 'fix/gh-184-leak',
    Diff: '+96 −12',
    Dirty: 'yes'
  },
  open: {
    view: 'editor',
    label: 'Open in editor'
  }
}, {
  id: 'brain',
  proj: 'cp',
  kind: 'brain',
  title: '3 evidence',
  subtitle: 'grounded @ 4f18a70',
  status: 'idle',
  x: 480,
  y: 380,
  detail: {
    Grounded: '@ 4f18a70',
    Evidence: '3 objects',
    Freshness: 'fresh'
  },
  open: {
    view: 'brain',
    label: 'Open Project Brain'
  }
}, /* RepoGraph Parser project */
{
  id: 'rgproj',
  proj: 'rg',
  kind: 'project',
  title: 'RepoGraph Parser',
  subtitle: 'org/repograph',
  status: 'active',
  x: 60,
  y: 150,
  detail: {
    Repo: 'org/repograph',
    Sessions: '1 active',
    PRs: '0 open',
    Workflow: 'none'
  },
  open: {
    view: 'command',
    label: 'Open project'
  }
}, {
  id: 'rgs',
  proj: 'rg',
  kind: 'session',
  title: '#71 · snapshot fix',
  subtitle: 'codex-cloud',
  status: 'failed',
  x: 300,
  y: 150,
  meta: ['29% ctx'],
  detail: {
    Harness: 'Codex Cloud',
    Profile: 'Codex Cloud GitHub',
    Branch: 'fix/snapshots',
    Checks: 'failing',
    Activity: '6m ago'
  },
  open: {
    view: 'terminal',
    sel: 's4',
    label: 'Open terminal'
  }
}, {
  id: 'rgwt',
  proj: 'rg',
  kind: 'worktree',
  title: 'fix/snapshots',
  status: 'active',
  x: 540,
  y: 150,
  meta: ['+12 −4'],
  detail: {
    Path: '~/wt/snap',
    Branch: 'fix/snapshots',
    Diff: '+12 −4',
    Dirty: 'yes'
  },
  open: {
    view: 'editor',
    label: 'Open in editor'
  }
}];
const EDGES = [['proj', 's1', 'active'], ['proj', 's2', 'active'], ['proj', 'team', 'active'], ['s1', 'wt1', 'active'], ['wt1', 'pr1', 'active'], ['s2', 'wt2', 'active'], ['team', 'brain', 'evidence'], ['rgproj', 'rgs', 'active'], ['rgs', 'rgwt', 'blocked']];
function ProjectGraph({
  sel,
  setSel,
  onInspect,
  project = 'cp',
  projectName = 'Project'
}) {
  const nodes = project === 'all' ? [] : NODES.filter(n => n.proj === project);
  const ids = new Set(nodes.map(n => n.id));
  const edges = EDGES.filter(([a, b]) => ids.has(a) && ids.has(b));
  const byId = Object.fromEntries(nodes.map(n => [n.id, n]));
  const isAll = project === 'all';
  const edgeColor = {
    active: 'var(--graph-edge-active)',
    evidence: 'var(--graph-edge-evidence)',
    blocked: 'var(--graph-edge-blocked)'
  };
  return /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'relative',
      height: '100%',
      background: 'var(--graph-canvas)',
      overflow: 'auto',
      backgroundImage: 'radial-gradient(var(--graph-grid) 1px, transparent 1px)',
      backgroundSize: '22px 22px'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'sticky',
      top: 0,
      zIndex: 5,
      display: 'flex',
      alignItems: 'center',
      gap: 10,
      padding: '12px 16px',
      background: 'linear-gradient(var(--graph-canvas), transparent)'
    }
  }, /*#__PURE__*/React.createElement("h1", {
    style: {
      margin: 0,
      font: 'var(--fw-semibold) var(--fs-h3)/1 var(--font-sans)'
    }
  }, "Project Graph"), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fs-meta) var(--font-mono)',
      color: 'var(--text-faint)'
    }
  }, projectName), /*#__PURE__*/React.createElement(GBadge, {
    tone: "neutral",
    mono: true
  }, nodes.length, " nodes"), /*#__PURE__*/React.createElement("div", {
    style: {
      marginLeft: 'auto',
      display: 'flex',
      gap: 6
    }
  }, /*#__PURE__*/React.createElement(GBtn, {
    variant: "ghost",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(GIco, {
      n: "filter"
    })
  }, "Filter"), /*#__PURE__*/React.createElement(GBtn, {
    variant: "secondary",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(GIco, {
      n: "maximize"
    })
  }, "Fit"))), nodes.length === 0 ? /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      inset: 0,
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      justifyContent: 'center',
      gap: 10,
      color: 'var(--text-muted)'
    }
  }, /*#__PURE__*/React.createElement(GIco, {
    n: "workflow",
    s: {
      width: 26,
      height: 26,
      color: 'var(--text-faint)'
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fw-medium) var(--fs-body) var(--font-sans)'
    }
  }, isAll ? 'The graph is per-project' : `No observability graph for ${projectName} yet`), /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fs-meta) var(--font-sans)',
      color: 'var(--text-faint)'
    }
  }, isAll ? 'Pick a single project from the switcher to see its graph.' : 'Start a session or run a workflow to populate it.')) : /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("svg", {
    style: {
      position: 'absolute',
      inset: 0,
      width: 920,
      height: 480,
      pointerEvents: 'none'
    }
  }, edges.map(([a, b, rel], i) => {
    const na = byId[a],
      nb = byId[b];
    const x1 = na.x + 84,
      y1 = na.y + 26,
      x2 = nb.x,
      y2 = nb.y + 26;
    const mx = (x1 + x2) / 2;
    return /*#__PURE__*/React.createElement("path", {
      key: i,
      d: `M${x1},${y1} C${mx},${y1} ${mx},${y2} ${x2},${y2}`,
      fill: "none",
      stroke: edgeColor[rel] || 'var(--graph-edge)',
      strokeWidth: "1.5",
      strokeDasharray: rel === 'evidence' ? '4 3' : undefined
    });
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'relative',
      width: 920,
      height: 480
    }
  }, nodes.map(n => /*#__PURE__*/React.createElement("div", {
    key: n.id,
    style: {
      position: 'absolute',
      left: n.x,
      top: n.y
    }
  }, /*#__PURE__*/React.createElement(GNode, {
    kind: n.kind,
    title: n.title,
    subtitle: n.subtitle,
    status: n.status,
    beacon: n.beacon,
    meta: n.meta,
    selected: sel === n.id,
    onClick: () => {
      setSel(n.id);
      onInspect && onInspect(n);
    }
  }))))));
}
const NODE_KIND_ICON = {
  project: 'folder-git-2',
  session: 'bot',
  team: 'users-round',
  worktree: 'folder-git-2',
  pr: 'git-pull-request',
  brain: 'brain'
};
window.NODE_KIND_ICON = NODE_KIND_ICON;

/* ---------------- Session Terminal ---------------- */
function SessionTerminal({
  sel,
  openGateway,
  overrides = {},
  team = false
}) {
  if (team) return /*#__PURE__*/React.createElement(TeamTerminal, {
    openGateway: openGateway
  });
  const base = KD2.sessions.find(x => x.id === sel) || KD2.sessions[0];
  const s = overrides[base.id] ? {
    ...base,
    status: overrides[base.id]
  } : base;
  const waiting = s.status === 'waiting-human';
  const lines = [{
    k: 'cmd',
    t: `/team-start backend --profile ${s.profile}`
  }, {
    k: 'out',
    t: 'resolved worktree ' + (s.worktree || '~/wt/session')
  }, {
    k: 'out',
    t: 'harness ' + s.harness + ' attached · context window 200k'
  }, {
    k: 'cmd',
    t: 'npm test -- --runInBand'
  }, {
    k: 'dim',
    t: '› PASS  src/gateway/risk.test.ts (12)'
  }, {
    k: 'dim',
    t: '› PASS  src/gateway/review.test.ts (8)'
  }, {
    k: waiting ? 'warn' : 'live',
    t: s.current
  }];
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      height: '100%',
      background: 'var(--surface-canvas)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '11px 16px',
      borderBottom: '1px solid var(--border-default)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      font: 'var(--fs-meta) var(--font-sans)',
      color: 'var(--text-faint)',
      marginBottom: 7
    }
  }, /*#__PURE__*/React.createElement("span", null, s.task ? s.task.id : 'Session'), /*#__PURE__*/React.createElement(GIco, {
    n: "chevron-right",
    s: {
      width: 12,
      height: 12
    }
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-secondary)'
    }
  }, s.branch)), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 10
    }
  }, /*#__PURE__*/React.createElement(GPill, {
    status: s.status,
    beacon: waiting,
    size: "md"
  }), /*#__PURE__*/React.createElement("h1", {
    style: {
      margin: 0,
      font: 'var(--fw-semibold) var(--fs-h3)/1 var(--font-sans)'
    }
  }, s.title), /*#__PURE__*/React.createElement("div", {
    style: {
      marginLeft: 'auto',
      display: 'flex',
      gap: 6
    }
  }, /*#__PURE__*/React.createElement(GBtn, {
    variant: "ghost",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(GIco, {
      n: "git-pull-request"
    })
  }, "Open PR"), /*#__PURE__*/React.createElement(GBtn, {
    variant: "secondary",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(GIco, {
      n: "pause"
    })
  }, "Pause"))), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 6,
      marginTop: 9,
      flexWrap: 'wrap'
    }
  }, /*#__PURE__*/React.createElement(GHarness, {
    harness: s.harness
  }), /*#__PURE__*/React.createElement(GProfile, {
    name: s.profile,
    provider: s.provider
  }), s.branch && /*#__PURE__*/React.createElement(GMeta, {
    tone: "branch"
  }, s.branch), s.worktree && /*#__PURE__*/React.createElement(GMeta, {
    tone: "worktree"
  }, s.worktree), s.pr && /*#__PURE__*/React.createElement(GMeta, {
    tone: "pr"
  }, s.pr))), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      overflowY: 'auto',
      padding: '12px 16px',
      background: 'var(--surface-sunken)',
      boxShadow: 'var(--elev-inset)',
      fontFamily: 'var(--font-mono)',
      fontSize: 'var(--fs-body)'
    }
  }, lines.map((l, i) => /*#__PURE__*/React.createElement(TermLine, {
    key: i,
    l: l
  })), waiting && /*#__PURE__*/React.createElement(PermissionPrompt, {
    s: s,
    openGateway: openGateway
  })));
}

/* ---------------- Team terminals (all panes at once) ---------------- */
function TeamTerminal({
  openGateway
}) {
  const t = KD2.team;
  const agents = [{
    id: 'lead',
    role: t.lead.role,
    harness: t.lead.harness,
    status: t.lead.status,
    lead: true,
    lines: [{
      k: 'cmd',
      t: '/team-start backend --profile ' + t.lead.profile
    }, {
      k: 'out',
      t: 'decomposing into 3 workstreams'
    }, {
      k: 'out',
      t: 'delegated → renderer, adapters, layout'
    }, {
      k: 'live',
      t: 'awaiting worker results'
    }]
  }, ...t.workers.map(w => ({
    id: w.id,
    role: w.role,
    harness: w.harness,
    status: w.status,
    wt: w.wt,
    lines: w.status === 'running' ? [{
      k: 'cmd',
      t: 'edit ' + w.task.replace('editing ', '')
    }, {
      k: 'dim',
      t: '› writing component tree'
    }, {
      k: 'live',
      t: w.task
    }] : w.status === 'waiting-perm' ? [{
      k: 'cmd',
      t: 'npm i d3-force'
    }, {
      k: 'warn',
      t: 'permission required — install dependency'
    }] : [{
      k: 'cmd',
      t: 'git push'
    }, {
      k: 'dim',
      t: '› opened PR #86'
    }, {
      k: 'out',
      t: w.task
    }]
  }))];
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      height: '100%',
      background: 'var(--surface-canvas)',
      minHeight: 0
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '11px 16px',
      borderBottom: '1px solid var(--border-default)',
      display: 'flex',
      alignItems: 'center',
      gap: 10
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--teal-ink)'
    }
  }, /*#__PURE__*/React.createElement(GIco, {
    n: "users-round"
  })), /*#__PURE__*/React.createElement("h1", {
    style: {
      margin: 0,
      font: 'var(--fw-semibold) var(--fs-h3)/1 var(--font-sans)'
    }
  }, t.name), /*#__PURE__*/React.createElement(GMeta, {
    tone: "branch"
  }, "team/phase-2-graph"), /*#__PURE__*/React.createElement(GBadge, {
    mono: true,
    style: {
      color: 'var(--text-muted)'
    }
  }, agents.length, " terminals"), /*#__PURE__*/React.createElement("div", {
    style: {
      marginLeft: 'auto',
      display: 'flex',
      gap: 6
    }
  }, /*#__PURE__*/React.createElement(GBtn, {
    variant: "ghost",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(GIco, {
      n: "layout-grid"
    })
  }, "Split"), /*#__PURE__*/React.createElement(GBtn, {
    variant: "secondary",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(GIco, {
      n: "git-merge"
    })
  }, "Integrate"))), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      minHeight: 0,
      display: 'grid',
      gridTemplateColumns: '1fr 1fr',
      gridTemplateRows: '1fr 1fr',
      gap: 1,
      background: 'var(--border-default)'
    }
  }, agents.map(a => /*#__PURE__*/React.createElement(AgentTermPane, {
    key: a.id,
    a: a,
    openGateway: openGateway
  }))));
}
function AgentTermPane({
  a,
  openGateway
}) {
  const waiting = a.status === 'waiting-perm';
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      minHeight: 0,
      background: 'var(--surface-canvas)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      padding: '7px 11px',
      background: 'var(--surface-panel)',
      borderBottom: '1px solid var(--border-subtle)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: a.lead ? 'var(--teal-ink)' : 'var(--text-faint)'
    }
  }, /*#__PURE__*/React.createElement(GIco, {
    n: a.lead ? 'workflow' : 'bot',
    s: {
      width: 14,
      height: 14
    }
  })), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fw-medium) var(--fs-label) var(--font-sans)',
      color: 'var(--text-primary)'
    }
  }, a.role), /*#__PURE__*/React.createElement(GPill, {
    status: a.status,
    size: "xs",
    beacon: waiting
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: 'auto'
    }
  }, /*#__PURE__*/React.createElement(GHarness, {
    harness: a.harness,
    showLabel: false
  }))), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      overflowY: 'auto',
      padding: '9px 11px',
      background: 'var(--surface-sunken)',
      boxShadow: 'var(--elev-inset)',
      fontFamily: 'var(--font-mono)',
      fontSize: 'var(--fs-meta)'
    }
  }, a.lines.map((l, i) => /*#__PURE__*/React.createElement(TermLine, {
    key: i,
    l: l
  })), waiting && /*#__PURE__*/React.createElement("div", {
    style: {
      marginTop: 9,
      border: '1px solid var(--attention-line)',
      background: 'var(--attention-surface)',
      borderRadius: 'var(--r-2)',
      padding: '8px 9px',
      fontFamily: 'var(--font-sans)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fw-medium) var(--fs-meta) var(--font-sans)',
      color: 'var(--attention-ink)',
      marginBottom: 7
    }
  }, "Permission \xB7 install d3-force"), /*#__PURE__*/React.createElement(GBtn, {
    variant: "attention",
    size: "xs",
    icon: /*#__PURE__*/React.createElement(GIco, {
      n: "check",
      s: {
        width: 12,
        height: 12
      }
    }),
    onClick: () => openGateway('q2')
  }, "Grant"))));
}
const TERM_COLORS = {
  cmd: 'var(--text-primary)',
  out: 'var(--text-secondary)',
  dim: 'var(--text-faint)',
  live: 'var(--live-ink)',
  warn: 'var(--attention-ink)'
};
function TermLine({
  l
}) {
  const prefix = l.k === 'cmd' ? '$ ' : l.k === 'live' ? '▶ ' : l.k === 'warn' ? '◆ ' : '';
  const prefixColor = l.k === 'cmd' ? 'var(--accent-ink)' : 'inherit';
  return /*#__PURE__*/React.createElement("div", {
    style: {
      color: TERM_COLORS[l.k],
      lineHeight: '22px',
      minHeight: 22,
      whiteSpace: 'pre-wrap',
      wordBreak: 'break-word'
    }
  }, prefix && /*#__PURE__*/React.createElement("span", {
    style: {
      color: prefixColor,
      animation: l.k === 'live' ? 'cp-live-pulse 1.6s var(--ease-inout) infinite' : undefined
    }
  }, prefix), l.t);
}
function PermissionPrompt({
  s,
  openGateway
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      marginTop: 12,
      border: '1px solid var(--attention-line)',
      background: 'var(--attention-surface)',
      borderRadius: 'var(--r-3)',
      padding: '12px 13px',
      fontFamily: 'var(--font-sans)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      marginBottom: 8
    }
  }, /*#__PURE__*/React.createElement(GRisk, {
    level: "medium"
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fw-semibold) var(--fs-body) var(--font-sans)',
      color: 'var(--attention-ink)'
    }
  }, "Permission required")), /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fs-body)/1.5 var(--font-sans)',
      color: 'var(--text-secondary)',
      marginBottom: 11
    }
  }, "Session wants to run ", /*#__PURE__*/React.createElement("code", {
    style: {
      font: 'var(--fs-meta) var(--font-mono)',
      color: 'var(--text-primary)',
      background: 'var(--surface-active)',
      padding: '1px 5px',
      borderRadius: 3
    }
  }, "npm test"), " \u2014 a sandboxed command in this worktree."), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 7
    }
  }, /*#__PURE__*/React.createElement(GBtn, {
    variant: "attention",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(GIco, {
      n: "check"
    }),
    onClick: () => openGateway('q1'),
    kbd: "\u23CE"
  }, "Approve once"), /*#__PURE__*/React.createElement(GBtn, {
    variant: "secondary",
    size: "sm"
  }, "Always allow tests"), /*#__PURE__*/React.createElement(GBtn, {
    variant: "ghost",
    size: "sm"
  }, "Deny")));
}

/* ---------------- Code & Delivery (Review · Worktrees · PRs) ---------------- */
function DiffReview({
  openBrain,
  project = 'cp',
  openGateway
}) {
  const [tab, setTab] = React.useState('Review');
  React.useEffect(() => {
    const t = setTimeout(() => window.lucide && window.lucide.createIcons(), 24);
    return () => clearTimeout(t);
  }, [tab]);
  const tabs = ['Review', 'Worktrees', 'Pull requests'];
  const counts = {
    Worktrees: (KD2.worktrees || []).filter(w => project === 'all' || w.proj === project).length,
    'Pull requests': (KD2.prs || []).filter(p => project === 'all' || p.proj === project).length
  };
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      height: '100%',
      background: 'var(--surface-canvas)',
      minHeight: 0
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 2,
      padding: '10px 16px 0',
      borderBottom: '1px solid var(--border-subtle)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-secondary)',
      marginRight: 8,
      display: 'inline-flex'
    }
  }, /*#__PURE__*/React.createElement(GIco, {
    n: "git-pull-request"
  })), tabs.map(t => /*#__PURE__*/React.createElement("button", {
    key: t,
    onClick: () => setTab(t),
    style: {
      padding: '8px 12px',
      border: 'none',
      background: 'transparent',
      cursor: 'pointer',
      font: `${tab === t ? 'var(--fw-semibold)' : 'var(--fw-medium)'} var(--fs-label) var(--font-sans)`,
      color: tab === t ? 'var(--accent-ink)' : 'var(--text-muted)',
      boxShadow: tab === t ? 'inset 0 -2px 0 var(--accent-solid)' : 'none'
    }
  }, t, counts[t] != null ? /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: 6,
      font: 'var(--fs-micro) var(--font-mono)',
      color: 'var(--text-faint)'
    }
  }, counts[t]) : null))), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      minHeight: 0
    }
  }, tab === 'Review' && /*#__PURE__*/React.createElement(ReviewTab, {
    openBrain: openBrain
  }), tab === 'Worktrees' && /*#__PURE__*/React.createElement(WorktreesTab, {
    project: project,
    openGateway: openGateway
  }), tab === 'Pull requests' && /*#__PURE__*/React.createElement(PRsTab, {
    project: project,
    openReview: () => setTab('Review'),
    openGateway: openGateway
  })));
}
function ReviewTab({
  openBrain
}) {
  const files = KD2.diff;
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      gridTemplateColumns: '220px 1fr',
      height: '100%',
      background: 'var(--surface-canvas)'
    }
  }, /*#__PURE__*/React.createElement("aside", {
    style: {
      borderRight: '1px solid var(--border-default)',
      background: 'var(--surface-panel)',
      overflowY: 'auto',
      padding: '12px 8px'
    }
  }, /*#__PURE__*/React.createElement(GEye, {
    style: {
      padding: '0 8px 10px'
    }
  }, "Changed files"), files.map((f, i) => /*#__PURE__*/React.createElement("div", {
    key: i,
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 7,
      padding: '6px 8px',
      borderRadius: 'var(--r-2)',
      background: i === 0 ? 'var(--surface-active)' : 'transparent',
      cursor: 'pointer',
      marginBottom: 2
    }
  }, /*#__PURE__*/React.createElement(GIco, {
    n: "file-code",
    s: {
      width: 13,
      height: 13,
      color: 'var(--text-faint)'
    }
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1,
      font: 'var(--fs-meta) var(--font-mono)',
      color: 'var(--text-secondary)',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap'
    }
  }, f.file.split('/').pop()), f.comments > 0 && /*#__PURE__*/React.createElement(GBadge, {
    tone: "review",
    size: "xs",
    mono: true
  }, f.comments))), /*#__PURE__*/React.createElement("div", {
    style: {
      marginTop: 'auto',
      padding: '10px 8px 0'
    }
  }, /*#__PURE__*/React.createElement(GBadge, {
    tone: "success",
    variant: "dot"
  }, "+476"), " ", /*#__PURE__*/React.createElement(GBadge, {
    tone: "danger",
    variant: "dot"
  }, "\u2212110"))), /*#__PURE__*/React.createElement("div", {
    style: {
      overflowY: 'auto',
      padding: '14px 16px'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 10,
      marginBottom: 14
    }
  }, /*#__PURE__*/React.createElement("h1", {
    style: {
      margin: 0,
      font: 'var(--fw-semibold) var(--fs-h3)/1 var(--font-sans)'
    }
  }, "Review \xB7 PR #84"), /*#__PURE__*/React.createElement(GPill, {
    status: "failing"
  }), /*#__PURE__*/React.createElement(GMeta, {
    tone: "pr"
  }, "#84"), /*#__PURE__*/React.createElement("div", {
    style: {
      marginLeft: 'auto',
      display: 'flex',
      gap: 6
    }
  }, /*#__PURE__*/React.createElement(GBtn, {
    variant: "secondary",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(GIco, {
      n: "brain"
    }),
    onClick: openBrain
  }, "Ask Brain"), /*#__PURE__*/React.createElement(GBtn, {
    variant: "primary",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(GIco, {
      n: "check"
    })
  }, "Approve PR"))), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 14
    }
  }, files.map((f, i) => /*#__PURE__*/React.createElement(GDiff, {
    key: i,
    file: f.file,
    header: f.header,
    lines: f.lines,
    comments: f.comments,
    onAsk: openBrain,
    onRequestFix: () => {}
  })))));
}

/* ---- Worktrees tab ---- */
const WT_STATUS = {
  dirty: {
    pill: 'dirty',
    label: 'Dirty'
  },
  active: {
    pill: 'active',
    label: 'Clean'
  },
  conflict: {
    pill: 'conflict',
    label: 'Conflict'
  }
};
function WorktreesTab({
  project,
  openGateway
}) {
  const rows = (KD2.worktrees || []).filter(w => project === 'all' || w.proj === project);
  return /*#__PURE__*/React.createElement("div", {
    style: {
      height: '100%',
      overflowY: 'auto',
      padding: '14px 16px'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 10,
      marginBottom: 12
    }
  }, /*#__PURE__*/React.createElement(GEye, null, "Worktrees"), /*#__PURE__*/React.createElement(GBadge, {
    mono: true,
    style: {
      color: 'var(--text-muted)'
    }
  }, rows.length), /*#__PURE__*/React.createElement("div", {
    style: {
      marginLeft: 'auto'
    }
  }, /*#__PURE__*/React.createElement(GBtn, {
    variant: "secondary",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(GIco, {
      n: "plus"
    })
  }, "New worktree"))), rows.length === 0 ? /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fs-label) var(--font-sans)',
      color: 'var(--text-faint)',
      padding: '16px 2px'
    }
  }, "No worktrees for this project.") : /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 8
    }
  }, rows.map(w => {
    const st = WT_STATUS[w.status] || WT_STATUS.active;
    return /*#__PURE__*/React.createElement("div", {
      key: w.id,
      style: {
        border: `1px solid ${w.status === 'conflict' ? 'var(--danger-line)' : 'var(--border-default)'}`,
        borderRadius: 'var(--r-3)',
        background: 'var(--surface-card)',
        padding: '11px 13px',
        display: 'flex',
        alignItems: 'center',
        gap: 12
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        width: 30,
        height: 30,
        flex: 'none',
        borderRadius: 'var(--r-2)',
        background: 'var(--surface-active)',
        color: 'var(--teal-ink)',
        display: 'inline-flex',
        alignItems: 'center',
        justifyContent: 'center'
      }
    }, /*#__PURE__*/React.createElement(GIco, {
      n: "folder-git-2",
      s: {
        width: 15,
        height: 15
      }
    })), /*#__PURE__*/React.createElement("div", {
      style: {
        minWidth: 0,
        flex: 1
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        flexWrap: 'wrap'
      }
    }, /*#__PURE__*/React.createElement("code", {
      style: {
        font: 'var(--fw-medium) var(--fs-label) var(--font-mono)',
        color: 'var(--text-primary)'
      }
    }, w.path), /*#__PURE__*/React.createElement(GPill, {
      status: st.pill,
      size: "xs",
      label: st.label
    }), w.dirty > 0 && /*#__PURE__*/React.createElement("span", {
      style: {
        font: 'var(--fs-meta) var(--font-mono)',
        color: 'var(--caution-ink)'
      }
    }, w.dirty, " changed")), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 6,
        marginTop: 5,
        flexWrap: 'wrap'
      }
    }, /*#__PURE__*/React.createElement(GMeta, {
      tone: "branch"
    }, w.branch), /*#__PURE__*/React.createElement("span", {
      style: {
        font: 'var(--fs-micro) var(--font-mono)',
        color: 'var(--text-faint)'
      }
    }, "\u2190 ", w.base), w.task && /*#__PURE__*/React.createElement(GMeta, {
      tone: w.task.tone === 'github' ? 'github' : w.task.tone === 'linear' ? 'linear' : 'accent',
      mono: false
    }, w.task.id), /*#__PURE__*/React.createElement(GMeta, {
      icon: /*#__PURE__*/React.createElement(GIco, {
        n: "git-commit-horizontal",
        s: {
          width: 12,
          height: 12
        }
      })
    }, w.commit), w.pr && /*#__PURE__*/React.createElement(GMeta, {
      tone: "pr"
    }, w.pr), w.checks && /*#__PURE__*/React.createElement(GPill, {
      status: w.checks === 'passing' ? 'passing' : 'failing',
      size: "xs"
    }))), /*#__PURE__*/React.createElement(GRisk, {
      level: w.risk
    }), w.status === 'conflict' ? /*#__PURE__*/React.createElement(GBtn, {
      variant: "danger",
      size: "sm",
      icon: /*#__PURE__*/React.createElement(GIco, {
        n: "git-merge",
        s: {
          width: 13,
          height: 13
        }
      }),
      onClick: () => openGateway && openGateway('q2')
    }, "Resolve") : /*#__PURE__*/React.createElement(GIconBtn, {
      icon: /*#__PURE__*/React.createElement(GIco, {
        n: "chevron-right",
        s: {
          width: 15,
          height: 15
        }
      }),
      size: "sm",
      "aria-label": "Open"
    }));
  })));
}

/* ---- Pull requests tab (lanes) ---- */
const LANES = [['open', 'Open', 'review'], ['ready', 'Ready to merge', 'success'], ['merged', 'Merged', 'review']];
function PRsTab({
  project,
  openReview,
  openGateway
}) {
  const prs = (KD2.prs || []).filter(p => project === 'all' || p.proj === project);
  return /*#__PURE__*/React.createElement("div", {
    style: {
      height: '100%',
      overflowY: 'auto',
      padding: '14px 16px'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 12,
      alignItems: 'flex-start',
      minWidth: 0
    }
  }, LANES.map(([lane, label, tone]) => {
    const items = prs.filter(p => p.lane === lane);
    return /*#__PURE__*/React.createElement("div", {
      key: lane,
      style: {
        flex: 1,
        minWidth: 0
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 7,
        padding: '0 2px 9px'
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        width: 7,
        height: 7,
        borderRadius: 999,
        background: `var(--${tone}-solid)`
      }
    }), /*#__PURE__*/React.createElement("span", {
      style: {
        font: 'var(--fw-semibold) var(--fs-micro) var(--font-sans)',
        letterSpacing: 'var(--tracking-caps)',
        textTransform: 'uppercase',
        color: 'var(--text-muted)'
      }
    }, label), /*#__PURE__*/React.createElement("span", {
      style: {
        font: '10px var(--font-mono)',
        color: 'var(--text-faint)'
      }
    }, items.length)), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        flexDirection: 'column',
        gap: 8
      }
    }, items.map(p => /*#__PURE__*/React.createElement("div", {
      key: p.id,
      onClick: openReview,
      style: {
        cursor: 'pointer',
        border: '1px solid var(--border-default)',
        borderRadius: 'var(--r-3)',
        background: 'var(--surface-card)',
        padding: '11px 12px',
        display: 'flex',
        flexDirection: 'column',
        gap: 8
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 7
      }
    }, /*#__PURE__*/React.createElement(GMeta, {
      tone: "pr"
    }, p.id), /*#__PURE__*/React.createElement(GPill, {
      status: p.checks === 'passing' ? 'passing' : 'failing',
      size: "xs"
    }), /*#__PURE__*/React.createElement("span", {
      style: {
        marginLeft: 'auto',
        font: 'var(--fs-micro) var(--font-mono)',
        color: 'var(--text-faint)'
      }
    }, p.age)), /*#__PURE__*/React.createElement("div", {
      style: {
        font: 'var(--fw-medium) var(--fs-label)/1.3 var(--font-sans)',
        color: 'var(--text-primary)'
      }
    }, p.title), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        font: 'var(--fs-micro) var(--font-mono)',
        color: 'var(--text-faint)'
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        color: 'var(--diff-add-ink)'
      }
    }, "+", p.adds), /*#__PURE__*/React.createElement("span", {
      style: {
        color: 'var(--diff-del-ink)'
      }
    }, "\u2212", p.dels), /*#__PURE__*/React.createElement("span", null, "\xB7 ", p.files, " files"), p.comments > 0 && /*#__PURE__*/React.createElement("span", null, "\xB7 \uD83D\uDDE9 ", p.comments)), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 6
      }
    }, /*#__PURE__*/React.createElement(GMeta, {
      tone: "branch"
    }, p.branch), lane === 'ready' && /*#__PURE__*/React.createElement(GBtn, {
      variant: "primary",
      size: "xs",
      icon: /*#__PURE__*/React.createElement(GIco, {
        n: "git-merge",
        s: {
          width: 12,
          height: 12
        }
      }),
      onClick: e => {
        e.stopPropagation();
        openGateway && openGateway('q2');
      },
      style: {
        marginLeft: 'auto'
      }
    }, "Merge")))), items.length === 0 && /*#__PURE__*/React.createElement("div", {
      style: {
        font: 'var(--fs-micro) var(--font-sans)',
        color: 'var(--text-faint)',
        padding: '6px 2px',
        border: '1px dashed var(--border-subtle)',
        borderRadius: 'var(--r-2)',
        textAlign: 'center'
      }
    }, "None")));
  })));
}
window.KitViews2 = {
  ProjectGraph,
  SessionTerminal,
  DiffReview
};
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/control-plane/kit-views2.jsx", error: String((e && e.message) || e) }); }

// ui_kits/control-plane/kit-views3.jsx
try { (() => {
/* ============================================================
   Control Plane UI kit — Editor (IDE) + Agent Team views.
   ============================================================ */
const _NS4 = window.ControlPlaneDesignSystem_a21911;
const {
  Button: EBtn,
  IconButton: EIconBtn,
  StatusPill: EPill,
  RiskBadge: ERisk,
  UsageMeter: EMeter,
  HarnessBadge: EHarness,
  ProfileBadge: EProfile,
  MetaChip: EMeta
} = _NS4;
const EBadge = _NS4.Badge || (({
  children,
  mono,
  style = {}
}) => /*#__PURE__*/React.createElement("span", {
  style: {
    font: `var(--fw-medium) 11px/1 ${mono ? 'var(--font-mono)' : 'var(--font-sans)'}`,
    ...style
  }
}, children));
const {
  Ico: EIco,
  Eyebrow: EEye
} = window.KitShell;
const KD4 = window.KIT;

/* ---- lightweight TS tokenizer for cosmetic highlighting ---- */
const KW = /\b(import|from|export|async|function|const|let|return|if|else|await|type|interface|new|class)\b/g;
const TYPE = /\b([A-Z][A-Za-z0-9]+)\b/g;
function hi(line) {
  if (line.trim().startsWith('//')) return [{
    c: 'var(--text-faint)',
    t: line
  }];
  const out = [];
  let i = 0;
  // naive: split on strings first
  const parts = line.split(/('[^']*')/g);
  parts.forEach(p => {
    if (p.startsWith("'") && p.endsWith("'")) {
      out.push({
        c: 'var(--success-ink)',
        t: p
      });
      return;
    }
    let last = 0;
    let m;
    const seg = p;
    const tokens = [];
    const re = new RegExp(KW.source + '|' + TYPE.source, 'g');
    while (m = re.exec(seg)) {
      if (m.index > last) tokens.push({
        c: 'var(--text-secondary)',
        t: seg.slice(last, m.index)
      });
      const isKw = KW.test(m[0]);
      KW.lastIndex = 0;
      tokens.push({
        c: isKw ? 'var(--accent-ink)' : 'var(--brain-ink)',
        t: m[0]
      });
      last = m.index + m[0].length;
    }
    if (last < seg.length) tokens.push({
      c: 'var(--text-secondary)',
      t: seg.slice(last)
    });
    out.push(...tokens);
  });
  return out;
}

/* ---------------- Editor / IDE ---------------- */
function EditorView({
  openGateway
}) {
  const {
    tree,
    files
  } = KD4;
  const markBg = {
    add: 'var(--diff-add-bg)',
    del: 'var(--diff-del-bg)',
    ctx: 'transparent'
  };
  const markGutter = {
    add: 'var(--diff-add-gutter)',
    del: 'var(--diff-del-gutter)'
  };
  const [active, setActive] = React.useState('review.ts');
  const [openTabs, setOpenTabs] = React.useState(['review.ts', 'risk.ts', 'gateway.test.ts']);
  React.useEffect(() => {
    setTimeout(() => window.lucide && window.lucide.createIcons(), 20);
  }, [active, openTabs]);
  const file = files[active] || files['review.ts'];
  const agentEdit = file.agentEdit;
  const problems = file.problems || [];
  const openFile = name => {
    if (!files[name]) return;
    setActive(name);
    setOpenTabs(o => o.includes(name) ? o : [...o, name]);
  };
  const closeTab = (e, name) => {
    e.stopPropagation();
    setOpenTabs(o => {
      const next = o.filter(n => n !== name);
      if (active === name && next.length) setActive(next[next.length - 1]);
      return next.length ? next : o;
    });
  };
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      gridTemplateColumns: '210px 1fr',
      gridTemplateRows: '1fr',
      height: '100%',
      background: 'var(--surface-canvas)',
      minHeight: 0
    }
  }, /*#__PURE__*/React.createElement("aside", {
    style: {
      borderRight: '1px solid var(--border-default)',
      background: 'var(--surface-panel)',
      overflowY: 'auto',
      padding: '10px 6px'
    }
  }, /*#__PURE__*/React.createElement(EEye, {
    style: {
      padding: '0 8px 8px',
      display: 'flex',
      alignItems: 'center',
      gap: 6
    }
  }, /*#__PURE__*/React.createElement(EIco, {
    n: "folder-git-2",
    s: {
      width: 12,
      height: 12
    }
  }), " control-plane"), tree.map((node, i) => /*#__PURE__*/React.createElement(TreeRow, {
    key: i,
    node: node,
    activeFile: active,
    onOpen: openFile
  }))), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      gridTemplateRows: '34px 1fr 150px',
      minWidth: 0,
      minHeight: 0
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'stretch',
      background: 'var(--surface-sunken)',
      borderBottom: '1px solid var(--border-default)',
      overflowX: 'auto'
    }
  }, openTabs.map(name => {
    const f = files[name];
    if (!f) return null;
    const on = name === active;
    return /*#__PURE__*/React.createElement("div", {
      key: name,
      onClick: () => setActive(name),
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 7,
        padding: '0 10px 0 12px',
        borderRight: '1px solid var(--border-subtle)',
        cursor: 'pointer',
        background: on ? 'var(--surface-canvas)' : 'transparent',
        color: on ? 'var(--text-primary)' : 'var(--text-muted)',
        boxShadow: on ? 'inset 0 2px 0 var(--accent-solid)' : 'none',
        font: 'var(--fs-meta) var(--font-mono)'
      }
    }, /*#__PURE__*/React.createElement(EIco, {
      n: "file-code",
      s: {
        width: 12,
        height: 12,
        color: f.agent ? 'var(--accent-ink)' : 'var(--text-faint)'
      }
    }), name, f.dirty ? /*#__PURE__*/React.createElement("span", {
      style: {
        width: 6,
        height: 6,
        borderRadius: 999,
        background: 'var(--caution-solid)'
      }
    }) : /*#__PURE__*/React.createElement("span", {
      onClick: e => closeTab(e, name),
      style: {
        display: 'inline-flex',
        borderRadius: 3
      }
    }, /*#__PURE__*/React.createElement(EIco, {
      n: "x",
      s: {
        width: 11,
        height: 11,
        color: 'var(--text-faint)'
      }
    })));
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      marginLeft: 'auto',
      display: 'flex',
      alignItems: 'center',
      gap: 4,
      padding: '0 8px'
    }
  }, file.agent && /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(EHarness, {
    harness: "claude-code",
    showLabel: false
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fs-micro) var(--font-mono)',
      color: 'var(--accent-ink)'
    }
  }, "live edit")))), /*#__PURE__*/React.createElement("div", {
    style: {
      overflow: 'auto',
      background: 'var(--surface-canvas)',
      position: 'relative',
      fontFamily: 'var(--font-mono)',
      fontSize: '13px'
    }
  }, file.lines.map((line, i) => {
    const mark = file.marks[i];
    const isAgent = agentEdit && agentEdit.line === i;
    return /*#__PURE__*/React.createElement("div", {
      key: i,
      style: {
        display: 'flex',
        alignItems: 'stretch',
        background: mark ? markBg[mark] : isAgent ? 'var(--accent-surface)' : 'transparent',
        minHeight: 22,
        lineHeight: '22px',
        position: 'relative'
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        width: 44,
        flex: 'none',
        textAlign: 'right',
        padding: '0 10px 0 0',
        color: 'var(--text-faint)',
        userSelect: 'none',
        boxShadow: mark && markGutter[mark] ? `inset -2px 0 0 ${markGutter[mark]}` : 'none'
      }
    }, i + 1), /*#__PURE__*/React.createElement("span", {
      style: {
        paddingLeft: 10,
        whiteSpace: 'pre',
        flex: 1,
        minWidth: 0
      }
    }, hi(line).map((tk, j) => /*#__PURE__*/React.createElement("span", {
      key: j,
      style: {
        color: tk.c
      }
    }, tk.t)), isAgent && /*#__PURE__*/React.createElement("span", {
      style: {
        display: 'inline-block',
        width: 2,
        height: 15,
        marginLeft: 1,
        background: 'var(--accent-solid)',
        verticalAlign: 'middle',
        animation: 'cp-live-pulse 1.1s steps(1) infinite'
      }
    })), isAgent && /*#__PURE__*/React.createElement("span", {
      style: {
        position: 'absolute',
        right: 8,
        top: 1,
        display: 'inline-flex',
        alignItems: 'center',
        gap: 5,
        height: 18,
        padding: '0 7px',
        borderRadius: 'var(--r-1)',
        background: 'var(--accent-solid)',
        color: 'var(--accent-on-solid)',
        font: 'var(--fw-medium) var(--fs-micro)/1 var(--font-sans)'
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        width: 5,
        height: 5,
        borderRadius: 999,
        background: 'currentColor',
        animation: 'cp-live-pulse 1.6s var(--ease-inout) infinite'
      }
    }), " ", agentEdit.who));
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      borderTop: '1px solid var(--border-default)',
      background: 'var(--surface-sunken)',
      display: 'flex',
      flexDirection: 'column',
      minHeight: 0
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 14,
      padding: '0 12px',
      height: 30,
      borderBottom: '1px solid var(--border-subtle)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fw-semibold) var(--fs-micro) var(--font-sans)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: 'var(--text-primary)',
      borderBottom: '2px solid var(--accent-solid)',
      height: 30,
      display: 'flex',
      alignItems: 'center'
    }
  }, agentEdit ? 'Agent activity' : 'Problems'), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fw-semibold) var(--fs-micro) var(--font-sans)',
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: 'var(--text-muted)'
    }
  }, "Problems ", /*#__PURE__*/React.createElement(EBadge, {
    mono: true,
    style: {
      color: problems.length ? 'var(--warning-ink)' : 'var(--text-faint)'
    }
  }, problems.length)), /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: 'auto',
      font: 'var(--fs-meta) var(--font-mono)',
      color: 'var(--text-faint)'
    }
  }, file.path, "/", active), agentEdit && /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(ERisk, {
    level: "medium"
  }), /*#__PURE__*/React.createElement(EBtn, {
    variant: "attention",
    size: "xs",
    icon: /*#__PURE__*/React.createElement(EIco, {
      n: "check",
      s: {
        width: 13,
        height: 13
      }
    }),
    onClick: openGateway
  }, "Approve edit"))), /*#__PURE__*/React.createElement("div", {
    style: {
      overflowY: 'auto',
      padding: '8px 12px',
      font: 'var(--fs-meta)/1.7 var(--font-mono)',
      color: 'var(--text-secondary)'
    }
  }, agentEdit ? /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--accent-ink)'
    }
  }, agentEdit.who), " \xB7 ", agentEdit.note), /*#__PURE__*/React.createElement("div", {
    style: {
      color: 'var(--text-faint)'
    }
  }, "\u21B3 proposing edits in ", file.path, "/", active), problems[0] && /*#__PURE__*/React.createElement("div", {
    style: {
      color: 'var(--warning-ink)'
    }
  }, "\u26A0 ", problems[0].text, " ", /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-faint)'
    }
  }, "(", problems[0].at, ")")), /*#__PURE__*/React.createElement("div", {
    style: {
      color: 'var(--text-faint)'
    }
  }, "awaiting approval to apply edit + run tests\u2026")) : problems.length ? problems.map((p, i) => /*#__PURE__*/React.createElement("div", {
    key: i,
    style: {
      color: p.sev === 'fail' ? 'var(--danger-ink)' : 'var(--warning-ink)'
    }
  }, p.sev === 'fail' ? '✕' : '⚠', " ", p.text, " ", /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-faint)'
    }
  }, "(", p.at, ")"))) : /*#__PURE__*/React.createElement("div", {
    style: {
      color: 'var(--text-faint)'
    }
  }, "No problems in this file.")))));
}
function TreeRow({
  node,
  activeFile,
  onOpen
}) {
  const isFile = node.type === 'file';
  const active = isFile && node.name === activeFile;
  const gitColor = {
    M: 'var(--caution-ink)',
    A: 'var(--success-ink)',
    D: 'var(--danger-ink)'
  }[node.git];
  return /*#__PURE__*/React.createElement("div", {
    onClick: () => isFile && onOpen(node.name),
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 6,
      height: 24,
      paddingLeft: 8 + node.depth * 13,
      paddingRight: 8,
      borderRadius: 'var(--r-1)',
      cursor: 'pointer',
      background: active ? 'var(--surface-active)' : 'transparent'
    }
  }, /*#__PURE__*/React.createElement(EIco, {
    n: isFile ? 'file-code' : node.open ? 'chevron-down' : 'chevron-right',
    s: {
      width: 13,
      height: 13,
      color: 'var(--text-faint)',
      flex: 'none'
    }
  }), !isFile && !node.open && /*#__PURE__*/React.createElement(EIco, {
    n: "folder",
    s: {
      width: 13,
      height: 13,
      color: 'var(--text-faint)',
      marginLeft: -2
    }
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1,
      font: `${isFile ? 'var(--fw-regular)' : 'var(--fw-medium)'} var(--fs-meta) var(--font-mono)`,
      color: active ? 'var(--text-primary)' : 'var(--text-secondary)',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap'
    }
  }, node.name), node.agent && /*#__PURE__*/React.createElement("span", {
    title: "agent editing",
    style: {
      width: 6,
      height: 6,
      borderRadius: 999,
      background: 'var(--accent-solid)',
      animation: 'cp-live-pulse 1.6s var(--ease-inout) infinite'
    }
  }), node.git && /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fs-micro) var(--font-mono)',
      color: gitColor,
      fontWeight: 'var(--fw-semibold)'
    }
  }, node.git));
}

/* ---------------- Agent Team ---------------- */
function AgentTeamView({
  openGateway,
  onOpenTerminals
}) {
  const t = KD4.team;
  return /*#__PURE__*/React.createElement("div", {
    style: {
      height: '100%',
      overflowY: 'auto',
      background: 'var(--surface-canvas)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '14px 16px',
      borderBottom: '1px solid var(--border-subtle)',
      display: 'flex',
      alignItems: 'center',
      gap: 10
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--teal-ink)'
    }
  }, /*#__PURE__*/React.createElement(EIco, {
    n: "users-round"
  })), /*#__PURE__*/React.createElement("h1", {
    style: {
      margin: 0,
      font: 'var(--fw-semibold) var(--fs-h3)/1 var(--font-sans)'
    }
  }, t.name), /*#__PURE__*/React.createElement(EBadge, {
    tone: "teal",
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      height: 18,
      padding: '0 6px',
      borderRadius: 'var(--r-1)',
      background: 'var(--teal-surface)',
      color: 'var(--teal-ink)'
    }
  }, t.pack), /*#__PURE__*/React.createElement("div", {
    style: {
      marginLeft: 'auto',
      display: 'flex',
      gap: 6
    }
  }, /*#__PURE__*/React.createElement(EBtn, {
    variant: "secondary",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(EIco, {
      n: "terminal"
    }),
    onClick: onOpenTerminals
  }, "Open terminals"), /*#__PURE__*/React.createElement(EBtn, {
    variant: "ghost",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(EIco, {
      n: "git-merge"
    })
  }, "Integrate"), /*#__PURE__*/React.createElement(EBtn, {
    variant: "ghost",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(EIco, {
      n: "pause"
    })
  }, "Pause team"))), /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '16px',
      maxWidth: 760
    }
  }, /*#__PURE__*/React.createElement(EEye, {
    style: {
      marginBottom: 8
    }
  }, "Orchestrator"), /*#__PURE__*/React.createElement("div", {
    onClick: onOpenTerminals,
    title: "Open terminals",
    style: {
      cursor: 'pointer',
      border: '1px solid var(--teal-line)',
      background: 'var(--teal-surface)',
      borderRadius: 'var(--r-3)',
      padding: '12px 13px',
      marginBottom: 6,
      display: 'flex',
      alignItems: 'center',
      gap: 11
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      width: 30,
      height: 30,
      flex: 'none',
      borderRadius: 'var(--r-2)',
      background: 'var(--teal-solid)',
      color: 'var(--teal-on-solid)',
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center'
    }
  }, /*#__PURE__*/React.createElement(EIco, {
    n: "workflow",
    s: {
      width: 16,
      height: 16
    }
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      minWidth: 0
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 8
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fw-semibold) var(--fs-body) var(--font-sans)'
    }
  }, t.lead.role), /*#__PURE__*/React.createElement(EPill, {
    status: t.lead.status,
    size: "xs"
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fs-meta) var(--font-mono)',
      color: 'var(--text-muted)',
      marginTop: 3
    }
  }, t.lead.task)), /*#__PURE__*/React.createElement(EHarness, {
    harness: t.lead.harness
  }), /*#__PURE__*/React.createElement(EMeter, {
    variant: "ring",
    size: "sm",
    value: t.lead.ctx,
    max: 200,
    label: "ctx"
  }), /*#__PURE__*/React.createElement(EIco, {
    n: "terminal",
    s: {
      width: 15,
      height: 15,
      color: 'var(--teal-ink)'
    }
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      height: 14,
      borderLeft: '1.5px solid var(--teal-line)',
      marginLeft: 24
    }
  }), /*#__PURE__*/React.createElement(EEye, {
    style: {
      marginBottom: 8
    }
  }, "Workers \xB7 ", t.workers.length), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 8
    }
  }, t.workers.map(w => /*#__PURE__*/React.createElement("div", {
    key: w.id,
    onClick: onOpenTerminals,
    title: "Open terminals",
    style: {
      cursor: 'pointer',
      position: 'relative',
      border: '1px solid var(--border-default)',
      background: 'var(--surface-card)',
      borderRadius: 'var(--r-3)',
      padding: '11px 12px',
      display: 'flex',
      alignItems: 'center',
      gap: 11,
      marginLeft: 24
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      position: 'absolute',
      left: -24,
      top: '50%',
      width: 24,
      height: 1,
      background: 'var(--teal-line)'
    }
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      width: 28,
      height: 28,
      flex: 'none',
      borderRadius: 'var(--r-2)',
      background: 'var(--surface-active)',
      color: 'var(--text-secondary)',
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center'
    }
  }, /*#__PURE__*/React.createElement(EIco, {
    n: "bot",
    s: {
      width: 15,
      height: 15
    }
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      minWidth: 0
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 8
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fw-medium) var(--fs-body) var(--font-sans)'
    }
  }, w.role), /*#__PURE__*/React.createElement(EPill, {
    status: w.status,
    size: "xs",
    beacon: w.status === 'waiting-perm'
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fs-meta) var(--font-mono)',
      color: 'var(--text-muted)',
      marginTop: 3
    }
  }, w.task, " \xB7 ", w.wt)), /*#__PURE__*/React.createElement(EHarness, {
    harness: w.harness,
    showLabel: false
  }), /*#__PURE__*/React.createElement(EMeter, {
    variant: "ring",
    size: "sm",
    value: w.ctx,
    max: 200,
    label: "ctx"
  }), w.status === 'waiting-perm' ? /*#__PURE__*/React.createElement(EBtn, {
    variant: "attention",
    size: "xs",
    onClick: e => {
      e.stopPropagation();
      openGateway();
    }
  }, "Grant") : /*#__PURE__*/React.createElement(EIconBtn, {
    icon: /*#__PURE__*/React.createElement(EIco, {
      n: "terminal",
      s: {
        width: 15,
        height: 15
      }
    }),
    size: "sm",
    "aria-label": "Open terminal",
    onClick: e => {
      e.stopPropagation();
      onOpenTerminals();
    }
  }))))));
}
window.KitViews3 = {
  EditorView,
  AgentTeamView
};
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/control-plane/kit-views3.jsx", error: String((e && e.message) || e) }); }

// ui_kits/control-plane/kit-views4.jsx
try { (() => {
/* ============================================================
   Control Plane UI kit — Project Brain (chat co-pilot),
   Audit timeline, Settings / Execution profiles.
   ============================================================ */
const _NS5 = window.ControlPlaneDesignSystem_a21911;
const {
  Button: BBtn,
  IconButton: BIconBtn,
  StatusPill: BPill,
  RiskBadge: BRisk,
  UsageMeter: BMeter,
  HarnessBadge: BHarness,
  ProfileBadge: BProfile,
  MetaChip: BMeta,
  EvidenceChip: BEvidence
} = _NS5;
const BBadge = _NS5.Badge || (({
  children,
  mono,
  style = {}
}) => /*#__PURE__*/React.createElement("span", {
  style: {
    font: `var(--fw-medium) 11px/1 ${mono ? 'var(--font-mono)' : 'var(--font-sans)'}`,
    ...style
  }
}, children));
const {
  Ico: BIco,
  Eyebrow: BEye
} = window.KitShell;
const KD5 = window.KIT;
const {
  useState: useS5,
  useRef: useR5,
  useEffect: useE5
} = React;

/* tiny markdown-ish: **bold** and `code` */
function rich(text) {
  const parts = [];
  const re = /(\*\*[^*]+\*\*|`[^`]+`)/g;
  let last = 0,
    m;
  while (m = re.exec(text)) {
    if (m.index > last) parts.push(text.slice(last, m.index));
    const tok = m[0];
    if (tok.startsWith('**')) parts.push(/*#__PURE__*/React.createElement("strong", {
      key: m.index,
      style: {
        color: 'var(--text-primary)',
        fontWeight: 'var(--fw-semibold)'
      }
    }, tok.slice(2, -2)));else parts.push(/*#__PURE__*/React.createElement("code", {
      key: m.index,
      style: {
        font: 'var(--fs-meta) var(--font-mono)',
        background: 'var(--surface-active)',
        padding: '1px 5px',
        borderRadius: 3,
        color: 'var(--brain-ink)'
      }
    }, tok.slice(1, -1)));
    last = m.index + tok.length;
  }
  if (last < text.length) parts.push(text.slice(last));
  return parts;
}

/* ---------------- Project Brain page (co-pilot) ---------------- */
const BRAIN_MODES = ['Ask', 'Plan', 'Review', 'Decisions', 'Memory'];
const SCOPES = ['Entire project', 'Current session', 'Current PR', 'Current plan task'];
function ProjectBrainPage({
  openGateway,
  drawer = false
}) {
  const [mode, setMode] = useS5('Ask');
  const [scope, setScope] = useS5('Entire project');
  const [thread, setThread] = useS5(KD5.brainThread);
  const [draft, setDraft] = useS5('');
  const [thinking, setThinking] = useS5(false);
  const scrollRef = useR5(null);
  useE5(() => {
    if (scrollRef.current) scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    setTimeout(() => window.lucide && window.lucide.createIcons(), 20);
  }, [thread, thinking]);
  const send = text => {
    const q = (text != null ? text : draft).trim();
    if (!q) return;
    setThread(t => [...t, {
      from: 'user',
      text: q
    }]);
    setDraft('');
    setThinking(true);
    setTimeout(() => {
      const reply = KD5.brainReplies.find(r => r.match.test(q)) || KD5.brainReplies[KD5.brainReplies.length - 1];
      setThinking(false);
      setThread(t => [...t, {
        from: 'brain',
        text: reply.text,
        evidence: reply.evidence,
        plan: reply.plan
      }]);
    }, 850);
  };
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      gridTemplateColumns: drawer ? '1fr' : '1fr 280px',
      height: '100%',
      background: 'var(--surface-canvas)',
      minHeight: 0
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      gridTemplateRows: 'auto 1fr auto',
      minHeight: 0,
      minWidth: 0
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '12px 16px 0',
      borderBottom: '1px solid var(--border-subtle)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 9,
      marginBottom: 12
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      width: 26,
      height: 26,
      borderRadius: 'var(--r-2)',
      background: 'var(--brain-surface)',
      border: '1px solid var(--brain-line)',
      color: 'var(--brain-ink)',
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center'
    }
  }, /*#__PURE__*/React.createElement(BIco, {
    n: "brain",
    s: {
      width: 15,
      height: 15
    }
  })), /*#__PURE__*/React.createElement("h1", {
    style: {
      margin: 0,
      font: 'var(--fw-semibold) var(--fs-h3)/1 var(--font-sans)'
    }
  }, "Project Brain"), /*#__PURE__*/React.createElement(BBadge, {
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 5,
      height: 18,
      padding: '0 7px',
      borderRadius: 999,
      background: 'var(--brain-surface)',
      color: 'var(--brain-ink)',
      font: 'var(--fs-micro) var(--font-mono)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      width: 5,
      height: 5,
      borderRadius: 999,
      background: 'var(--brain-solid)'
    }
  }), " grounded \xB7 142 objects"), /*#__PURE__*/React.createElement("div", {
    style: {
      marginLeft: 'auto',
      display: 'flex',
      gap: 6,
      alignItems: 'center'
    }
  }, /*#__PURE__*/React.createElement(BIco, {
    n: "search",
    s: {
      width: 14,
      height: 14,
      color: 'var(--text-faint)'
    }
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fs-meta) var(--font-mono)',
      color: 'var(--text-faint)'
    }
  }, "scope:"), /*#__PURE__*/React.createElement("select", {
    value: scope,
    onChange: e => setScope(e.target.value),
    style: {
      background: 'var(--surface-input)',
      color: 'var(--text-secondary)',
      border: '1px solid var(--border-default)',
      borderRadius: 'var(--r-1)',
      font: 'var(--fs-meta) var(--font-sans)',
      padding: '2px 6px'
    }
  }, SCOPES.map(s => /*#__PURE__*/React.createElement("option", {
    key: s
  }, s))))), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 2
    }
  }, BRAIN_MODES.map(m => /*#__PURE__*/React.createElement("button", {
    key: m,
    onClick: () => setMode(m),
    style: {
      padding: '7px 11px',
      border: 'none',
      background: 'transparent',
      cursor: 'pointer',
      font: `${mode === m ? 'var(--fw-semibold)' : 'var(--fw-medium)'} var(--fs-label) var(--font-sans)`,
      color: mode === m ? 'var(--brain-ink)' : 'var(--text-muted)',
      boxShadow: mode === m ? 'inset 0 -2px 0 var(--brain-solid)' : 'none'
    }
  }, m)))), /*#__PURE__*/React.createElement("div", {
    ref: scrollRef,
    style: {
      overflowY: 'auto',
      padding: '16px',
      display: 'flex',
      flexDirection: 'column',
      gap: 16
    }
  }, thread.map((m, i) => m.from === 'user' ? /*#__PURE__*/React.createElement(UserMsg, {
    key: i,
    text: m.text
  }) : /*#__PURE__*/React.createElement(BrainMsg, {
    key: i,
    m: m,
    openGateway: openGateway,
    send: send
  })), thinking && /*#__PURE__*/React.createElement(ThinkingMsg, null)), /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '12px 16px',
      borderTop: '1px solid var(--border-default)',
      background: 'var(--surface-panel)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 7,
      marginBottom: 9,
      flexWrap: 'wrap'
    }
  }, ['Start the next backend task', 'Why are checks failing?', 'Summarize today'].map(s => /*#__PURE__*/React.createElement("button", {
    key: s,
    onClick: () => send(s),
    style: {
      padding: '4px 9px',
      borderRadius: 999,
      border: '1px solid var(--border-default)',
      background: 'var(--surface-card)',
      color: 'var(--text-secondary)',
      font: 'var(--fs-meta) var(--font-sans)',
      cursor: 'pointer'
    }
  }, s))), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'flex-end',
      gap: 8,
      background: 'var(--surface-input)',
      border: '1px solid var(--border-strong)',
      borderRadius: 'var(--r-3)',
      padding: '8px 8px 8px 12px'
    }
  }, /*#__PURE__*/React.createElement("textarea", {
    value: draft,
    onChange: e => setDraft(e.target.value),
    rows: 1,
    placeholder: `Ask Project Brain to ${mode === 'Plan' ? 'plan an action' : 'explain, plan, or recall…'}`,
    onKeyDown: e => {
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        send();
      }
    },
    style: {
      flex: 1,
      resize: 'none',
      background: 'transparent',
      border: 'none',
      outline: 'none',
      color: 'var(--text-primary)',
      font: 'var(--fs-body)/1.5 var(--font-sans)',
      maxHeight: 90
    }
  }), /*#__PURE__*/React.createElement(BBtn, {
    variant: "primary",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(BIco, {
      n: "arrow-up",
      s: {
        width: 14,
        height: 14
      }
    }),
    onClick: () => send()
  }, "Ask")))), !drawer && /*#__PURE__*/React.createElement("aside", {
    style: {
      borderLeft: '1px solid var(--border-default)',
      background: 'var(--surface-panel)',
      overflowY: 'auto',
      padding: '14px'
    }
  }, /*#__PURE__*/React.createElement(BEye, {
    style: {
      marginBottom: 10
    }
  }, "Memory & decisions"), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 8
    }
  }, KD5.brainMemory.map((m, i) => /*#__PURE__*/React.createElement("div", {
    key: i,
    style: {
      border: '1px solid var(--border-subtle)',
      borderRadius: 'var(--r-2)',
      padding: '9px 10px',
      background: 'var(--surface-card)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 6,
      marginBottom: 4
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: m.kind === 'decision' ? 'var(--brain-ink)' : 'var(--text-faint)'
    }
  }, /*#__PURE__*/React.createElement(BIco, {
    n: m.kind === 'decision' ? 'gavel' : m.kind === 'anchor' ? 'anchor' : 'database',
    s: {
      width: 13,
      height: 13
    }
  })), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fw-medium) var(--fs-label) var(--font-sans)',
      color: 'var(--text-primary)'
    }
  }, m.label), /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: 'auto',
      font: 'var(--fs-micro) var(--font-mono)',
      color: m.t === 'fresh' ? 'var(--success-ink)' : 'var(--text-faint)'
    }
  }, m.t)), /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fs-meta) var(--font-sans)',
      color: 'var(--text-muted)'
    }
  }, m.sub))))));
}
function UserMsg({
  text
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      justifyContent: 'flex-end'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      maxWidth: '76%',
      background: 'var(--accent-surface)',
      border: '1px solid var(--accent-line)',
      borderRadius: 'var(--r-3) var(--r-3) var(--r-1) var(--r-3)',
      padding: '9px 12px',
      font: 'var(--fs-body)/1.5 var(--font-sans)',
      color: 'var(--text-primary)'
    }
  }, text));
}
function ThinkingMsg() {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 10
    }
  }, /*#__PURE__*/React.createElement(BrainAvatar, null), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 6,
      padding: '9px 12px',
      borderRadius: 'var(--r-3)',
      background: 'var(--surface-card)',
      border: '1px solid var(--border-subtle)'
    }
  }, [0, 1, 2].map(i => /*#__PURE__*/React.createElement("span", {
    key: i,
    style: {
      width: 6,
      height: 6,
      borderRadius: 999,
      background: 'var(--brain-ink)',
      animation: `cp-live-pulse 1s ${i * 0.18}s var(--ease-inout) infinite`
    }
  })), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fs-meta) var(--font-mono)',
      color: 'var(--text-faint)',
      marginLeft: 4
    }
  }, "retrieving evidence\u2026")));
}
function BrainAvatar() {
  return /*#__PURE__*/React.createElement("span", {
    style: {
      width: 26,
      height: 26,
      flex: 'none',
      borderRadius: 'var(--r-2)',
      background: 'var(--brain-surface)',
      border: '1px solid var(--brain-line)',
      color: 'var(--brain-ink)',
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center'
    }
  }, /*#__PURE__*/React.createElement(BIco, {
    n: "brain",
    s: {
      width: 14,
      height: 14
    }
  }));
}
function BrainMsg({
  m,
  openGateway,
  send
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 10
    }
  }, /*#__PURE__*/React.createElement(BrainAvatar, null), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      minWidth: 0
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      background: 'var(--surface-card)',
      border: '1px solid var(--border-subtle)',
      borderRadius: 'var(--r-3) var(--r-3) var(--r-3) var(--r-1)',
      padding: '11px 13px'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fs-body)/1.6 var(--font-sans)',
      color: 'var(--text-secondary)'
    }
  }, rich(m.text)), m.evidence && /*#__PURE__*/React.createElement("div", {
    style: {
      marginTop: 11
    }
  }, /*#__PURE__*/React.createElement(BEye, {
    style: {
      marginBottom: 7
    }
  }, "Evidence"), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexWrap: 'wrap',
      gap: 6
    }
  }, m.evidence.map((e, i) => /*#__PURE__*/React.createElement(BEvidence, {
    key: i,
    kind: e.kind,
    label: e.label,
    sub: e.sub,
    freshness: e.freshness
  })))), m.plan && /*#__PURE__*/React.createElement("div", {
    style: {
      marginTop: 12,
      border: '1px solid var(--brain-line)',
      borderRadius: 'var(--r-2)',
      overflow: 'hidden',
      background: 'var(--brain-surface)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 7,
      padding: '8px 11px',
      borderBottom: '1px solid var(--brain-line)'
    }
  }, /*#__PURE__*/React.createElement(BIco, {
    n: "list-checks",
    s: {
      width: 14,
      height: 14,
      color: 'var(--brain-ink)'
    }
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fw-semibold) var(--fs-label) var(--font-sans)',
      color: 'var(--brain-ink)'
    }
  }, m.plan.title), /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: 'auto',
      font: 'var(--fs-micro) var(--font-mono)',
      color: 'var(--text-muted)'
    }
  }, m.plan.steps.length, " steps")), m.plan.steps.map((s, i) => /*#__PURE__*/React.createElement("div", {
    key: i,
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 9,
      padding: '8px 11px',
      borderBottom: i < m.plan.steps.length - 1 ? '1px solid var(--brain-line)' : 'none',
      background: 'var(--surface-card)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      width: 17,
      height: 17,
      flex: 'none',
      borderRadius: 999,
      background: 'var(--surface-active)',
      color: 'var(--text-muted)',
      font: 'var(--fw-semibold) var(--fs-micro) var(--font-mono)',
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center'
    }
  }, i + 1), /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1,
      font: 'var(--fs-label)/1.35 var(--font-sans)',
      color: 'var(--text-secondary)'
    }
  }, s.text), /*#__PURE__*/React.createElement(BRisk, {
    level: s.risk
  }))), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 7,
      padding: '10px 11px',
      background: 'var(--surface-card)'
    }
  }, /*#__PURE__*/React.createElement(BBtn, {
    variant: "primary",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(BIco, {
      n: "shield-check",
      s: {
        width: 14,
        height: 14
      }
    }),
    onClick: () => openGateway('edit')
  }, "Run via Gateway"), /*#__PURE__*/React.createElement(BBtn, {
    variant: "ghost",
    size: "sm"
  }, "Edit plan"))))));
}

/* ---------------- Audit timeline ---------------- */
const AUDIT_ICON = {
  approval: 'shield-check',
  gateway: 'shield-x',
  git: 'git-commit-horizontal',
  session: 'circle-dot',
  brain: 'brain',
  pr: 'git-pull-request',
  workflow: 'workflow',
  profile: 'key-round'
};
const AUDIT_GROUP = {
  Approvals: ['approval', 'gateway'],
  Git: ['git', 'pr'],
  Sessions: ['session'],
  Brain: ['brain'],
  Workflow: ['workflow']
};
function AuditTimeline({
  project = 'cp',
  projectName = 'Project',
  allProjects
}) {
  const [filter, setFilter] = useS5('All');
  const [scope, setScope] = useS5('project');
  const rows = KD5.audit.filter(e => (scope === 'all' || project === 'all' || e.proj === project) && (filter === 'All' || (AUDIT_GROUP[filter] || []).includes(e.kind)));
  const projTotal = KD5.audit.filter(e => e.proj === project).length;
  return /*#__PURE__*/React.createElement("div", {
    style: {
      height: '100%',
      overflowY: 'auto',
      background: 'var(--surface-canvas)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'sticky',
      top: 0,
      zIndex: 5,
      padding: '14px 16px 10px',
      background: 'var(--surface-canvas)',
      borderBottom: '1px solid var(--border-subtle)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 10,
      marginBottom: 12
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--slate-ink)'
    }
  }, /*#__PURE__*/React.createElement(BIco, {
    n: "scroll-text"
  })), /*#__PURE__*/React.createElement("h1", {
    style: {
      margin: 0,
      font: 'var(--fw-semibold) var(--fs-h3)/1 var(--font-sans)'
    }
  }, "Audit trail"), /*#__PURE__*/React.createElement(BBadge, {
    mono: true,
    style: {
      color: 'var(--text-muted)'
    }
  }, "append-only \xB7 ", rows.length, " events"), /*#__PURE__*/React.createElement("div", {
    style: {
      marginLeft: 'auto',
      display: 'flex',
      gap: 6,
      alignItems: 'center'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      borderRadius: 'var(--r-1)',
      overflow: 'hidden',
      border: '1px solid var(--border-default)'
    }
  }, /*#__PURE__*/React.createElement("button", {
    onClick: () => setScope('project'),
    style: {
      padding: '4px 9px',
      border: 'none',
      cursor: 'pointer',
      font: 'var(--fw-medium) var(--fs-meta) var(--font-sans)',
      background: scope === 'project' ? 'var(--accent-surface)' : 'transparent',
      color: scope === 'project' ? 'var(--accent-ink)' : 'var(--text-muted)'
    }
  }, projectName.length > 18 ? projectName.slice(0, 18) + '…' : projectName), /*#__PURE__*/React.createElement("button", {
    onClick: () => setScope('all'),
    style: {
      padding: '4px 9px',
      border: 'none',
      borderLeft: '1px solid var(--border-default)',
      cursor: 'pointer',
      font: 'var(--fw-medium) var(--fs-meta) var(--font-sans)',
      background: scope === 'all' ? 'var(--accent-surface)' : 'transparent',
      color: scope === 'all' ? 'var(--accent-ink)' : 'var(--text-muted)'
    }
  }, "All projects")), /*#__PURE__*/React.createElement(BBtn, {
    variant: "ghost",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(BIco, {
      n: "download"
    })
  }, "Export"))), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 6
    }
  }, KD5.auditFilters.map(f => /*#__PURE__*/React.createElement("button", {
    key: f,
    onClick: () => setFilter(f),
    style: {
      padding: '4px 10px',
      borderRadius: 999,
      cursor: 'pointer',
      border: `1px solid ${filter === f ? 'var(--accent-line)' : 'var(--border-default)'}`,
      background: filter === f ? 'var(--accent-surface)' : 'transparent',
      color: filter === f ? 'var(--accent-ink)' : 'var(--text-muted)',
      font: 'var(--fw-medium) var(--fs-meta) var(--font-sans)'
    }
  }, f)))), /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '8px 16px 24px',
      maxWidth: 760
    }
  }, rows.length === 0 ? /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fs-label) var(--font-sans)',
      color: 'var(--text-muted)',
      padding: '20px 4px',
      display: 'flex',
      alignItems: 'center',
      gap: 8
    }
  }, /*#__PURE__*/React.createElement(BIco, {
    n: "scroll-text",
    s: {
      width: 15,
      height: 15,
      color: 'var(--text-faint)'
    }
  }), " No audit events for this project yet.") : rows.map((e, i) => /*#__PURE__*/React.createElement(AuditRow, {
    key: i,
    e: e,
    last: i === rows.length - 1
  }))));
}
function AuditRow({
  e,
  last
}) {
  const resultColor = {
    approved: 'var(--success-ink)',
    denied: 'var(--danger-ink)',
    pending: 'var(--attention-ink)'
  }[e.result];
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 12,
      position: 'relative'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      flex: 'none',
      width: 26
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      width: 26,
      height: 26,
      borderRadius: 999,
      background: 'var(--surface-card)',
      border: '1px solid var(--border-default)',
      color: e.kind === 'brain' ? 'var(--brain-ink)' : resultColor || 'var(--text-muted)',
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      zIndex: 1
    }
  }, /*#__PURE__*/React.createElement(BIco, {
    n: AUDIT_ICON[e.kind] || 'dot',
    s: {
      width: 13,
      height: 13
    }
  })), !last && /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1,
      width: 1,
      background: 'var(--border-subtle)',
      minHeight: 14
    }
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      minWidth: 0,
      paddingBottom: 16
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      flexWrap: 'wrap'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fw-medium) var(--fs-body) var(--font-sans)',
      color: 'var(--text-primary)'
    }
  }, e.text), e.risk && /*#__PURE__*/React.createElement(BRisk, {
    level: e.risk
  }), e.result && e.result !== 'pending' && /*#__PURE__*/React.createElement(BBadge, {
    style: {
      color: resultColor
    }
  }, e.result)), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      marginTop: 3,
      font: 'var(--fs-meta) var(--font-mono)',
      color: 'var(--text-faint)'
    }
  }, /*#__PURE__*/React.createElement("span", null, e.t), /*#__PURE__*/React.createElement("span", null, "\xB7"), /*#__PURE__*/React.createElement("span", {
    style: {
      color: e.actor === 'You' ? 'var(--accent-ink)' : 'var(--text-muted)'
    }
  }, e.actor), e.target && /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("span", null, "\u2192"), /*#__PURE__*/React.createElement("span", null, e.target)), e.meta && /*#__PURE__*/React.createElement(BMeta, {
    tone: e.kind === 'pr' ? 'pr' : 'branch',
    style: {
      marginLeft: 2
    }
  }, e.meta))));
}

/* ---------------- Settings / Execution profiles ---------------- */
const HEALTH = {
  active: {
    pill: 'completed',
    label: 'Active'
  },
  available: {
    pill: 'idle',
    label: 'Available'
  },
  'rate-limited': {
    pill: 'stale',
    label: 'Rate-limited'
  },
  'auth-expired': {
    pill: 'failed',
    label: 'Auth expired'
  }
};
function SettingsProfiles() {
  const [sec, setSec] = useS5('Integrations');
  useE5(() => {
    const t = setTimeout(() => window.lucide && window.lucide.createIcons(), 24);
    return () => clearTimeout(t);
  }, [sec]);
  return /*#__PURE__*/React.createElement("div", {
    style: {
      height: '100%',
      overflowY: 'auto',
      background: 'var(--surface-canvas)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '14px 16px 0',
      borderBottom: '1px solid var(--border-subtle)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 10,
      marginBottom: 12
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-secondary)'
    }
  }, /*#__PURE__*/React.createElement(BIco, {
    n: "settings"
  })), /*#__PURE__*/React.createElement("h1", {
    style: {
      margin: 0,
      font: 'var(--fw-semibold) var(--fs-h3)/1 var(--font-sans)'
    }
  }, "Settings")), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 2
    }
  }, ['Integrations', 'Execution profiles', 'Usage', 'Security & policy'].map(t => /*#__PURE__*/React.createElement("button", {
    key: t,
    onClick: () => setSec(t),
    style: {
      padding: '8px 12px',
      border: 'none',
      background: 'transparent',
      cursor: 'pointer',
      font: `${sec === t ? 'var(--fw-semibold)' : 'var(--fw-medium)'} var(--fs-label) var(--font-sans)`,
      color: sec === t ? 'var(--accent-ink)' : 'var(--text-muted)',
      boxShadow: sec === t ? 'inset 0 -2px 0 var(--accent-solid)' : 'none'
    }
  }, t)))), sec === 'Integrations' && /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '16px',
      maxWidth: 760,
      display: 'flex',
      flexDirection: 'column',
      gap: 10
    }
  }, KD5.integrations.map(it => /*#__PURE__*/React.createElement("div", {
    key: it.id,
    style: {
      border: `1px solid ${it.connected ? 'var(--border-default)' : 'var(--caution-line)'}`,
      borderRadius: 'var(--r-3)',
      background: it.connected ? 'var(--surface-card)' : 'var(--caution-surface)',
      padding: '13px 14px',
      display: 'flex',
      alignItems: 'center',
      gap: 13
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      width: 34,
      height: 34,
      flex: 'none',
      borderRadius: 'var(--r-2)',
      background: 'var(--surface-active)',
      color: it.connected ? 'var(--text-secondary)' : 'var(--caution-ink)',
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center'
    }
  }, /*#__PURE__*/React.createElement(BIco, {
    n: it.icon,
    s: {
      width: 17,
      height: 17
    }
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      minWidth: 0
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 8
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fw-semibold) var(--fs-body) var(--font-sans)'
    }
  }, it.name), /*#__PURE__*/React.createElement(BPill, {
    status: it.connected ? 'completed' : 'idle',
    size: "xs",
    label: it.connected ? 'Connected' : 'Not connected'
  }), it.scope && /*#__PURE__*/React.createElement(BBadge, {
    mono: true,
    style: {
      color: 'var(--text-faint)'
    }
  }, it.scope)), /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fs-meta) var(--font-sans)',
      color: 'var(--text-muted)',
      marginTop: 3
    }
  }, it.detail)), it.connected ? /*#__PURE__*/React.createElement(BBtn, {
    variant: "secondary",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(BIco, {
      n: "settings-2",
      s: {
        width: 13,
        height: 13
      }
    })
  }, it.action) : /*#__PURE__*/React.createElement(BBtn, {
    variant: "attention",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(BIco, {
      n: "plug",
      s: {
        width: 13,
        height: 13
      }
    })
  }, it.action))), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 9,
      padding: '11px 13px',
      borderRadius: 'var(--r-3)',
      border: '1px dashed var(--border-default)',
      color: 'var(--text-muted)',
      font: 'var(--fs-meta)/1.5 var(--font-sans)'
    }
  }, /*#__PURE__*/React.createElement(BIco, {
    n: "git-compare",
    s: {
      width: 15,
      height: 15,
      color: 'var(--text-faint)'
    }
  }), "Linking ", /*#__PURE__*/React.createElement("strong", {
    style: {
      color: 'var(--text-secondary)'
    }
  }, "GitHub \u2194 Linear"), " lets a Linear ticket and its GitHub issue/PR resolve to one task in the inbox. Connect Linear to enable bi-directional mapping.")), sec === 'Execution profiles' && /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '16px',
      maxWidth: 800,
      display: 'flex',
      flexDirection: 'column',
      gap: 10
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 10
    }
  }, /*#__PURE__*/React.createElement(BBadge, {
    mono: true,
    style: {
      color: 'var(--text-muted)'
    }
  }, KD5.profilesDetail.length, " configured"), /*#__PURE__*/React.createElement("div", {
    style: {
      marginLeft: 'auto'
    }
  }, /*#__PURE__*/React.createElement(BBtn, {
    variant: "primary",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(BIco, {
      n: "plus"
    })
  }, "Add profile"))), KD5.profilesDetail.map((p, i) => {
    const h = HEALTH[p.health] || HEALTH.available;
    const over = p.usage >= 95;
    return /*#__PURE__*/React.createElement("div", {
      key: i,
      style: {
        border: `1px solid ${over ? 'var(--warning-line)' : 'var(--border-default)'}`,
        borderRadius: 'var(--r-3)',
        background: 'var(--surface-card)',
        padding: '13px 14px',
        display: 'grid',
        gridTemplateColumns: 'auto 1fr 220px auto',
        alignItems: 'center',
        gap: 14
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        width: 34,
        height: 34,
        flex: 'none',
        borderRadius: 'var(--r-2)',
        background: p.provider === 'claude' ? 'var(--domain-claude-surface)' : 'var(--domain-codex-surface)',
        display: 'inline-flex',
        alignItems: 'center',
        justifyContent: 'center',
        font: 'var(--font-mono)',
        color: p.provider === 'claude' ? 'var(--domain-claude)' : 'var(--domain-codex)',
        fontSize: 16
      }
    }, p.provider === 'claude' ? '✻' : '⌁'), /*#__PURE__*/React.createElement("div", {
      style: {
        minWidth: 0
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 8
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        font: 'var(--fw-semibold) var(--fs-body) var(--font-sans)'
      }
    }, p.name), /*#__PURE__*/React.createElement(BPill, {
      status: h.pill,
      size: "xs",
      label: h.label
    })), /*#__PURE__*/React.createElement("div", {
      style: {
        font: 'var(--fs-meta) var(--font-sans)',
        color: 'var(--text-muted)',
        marginTop: 3
      }
    }, p.note, " \xB7 ", p.sessions, " active session", p.sessions === 1 ? '' : 's')), /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement(BMeter, {
      label: "Quota",
      value: p.usage,
      max: p.limit,
      valueText: `${p.usage} / ${p.limit}${p.resets !== '—' ? ' · resets ' + p.resets : ''}`
    })), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        gap: 6,
        justifyContent: 'flex-end'
      }
    }, p.health === 'auth-expired' ? /*#__PURE__*/React.createElement(BBtn, {
      variant: "attention",
      size: "sm",
      icon: /*#__PURE__*/React.createElement(BIco, {
        n: "refresh-cw",
        s: {
          width: 13,
          height: 13
        }
      })
    }, "Re-auth") : /*#__PURE__*/React.createElement(BIconBtn, {
      icon: /*#__PURE__*/React.createElement(BIco, {
        n: "settings-2",
        s: {
          width: 15,
          height: 15
        }
      }),
      size: "sm",
      "aria-label": "Configure"
    })));
  })), sec === 'Usage' && /*#__PURE__*/React.createElement(UsageSection, null), sec === 'Security & policy' && /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '16px',
      maxWidth: 760,
      display: 'flex',
      flexDirection: 'column',
      gap: 16
    }
  }, /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement(BEye, {
    style: {
      marginBottom: 10
    }
  }, "Default approval policy"), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 7
    }
  }, [['shield-check', 'Read-only & low risk', 'Auto-approved — no confirmation', 'readonly'], ['shield', 'Medium risk', 'Confirm each action (create worktree, send to agent, draft PR)', 'medium'], ['shield-alert', 'High risk', 'Explicit confirmation (git writes, run commands, ticket changes)', 'high'], ['shield-x', 'Critical', 'Typed confirmation · no automation (force-push, delete, secrets)', 'critical']].map(([ic, name, desc, lvl], i) => /*#__PURE__*/React.createElement("div", {
    key: i,
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 11,
      padding: '11px 12px',
      borderRadius: 'var(--r-2)',
      border: '1px solid var(--border-default)',
      background: 'var(--surface-card)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--risk-' + lvl + ')'
    }
  }, /*#__PURE__*/React.createElement(BIco, {
    n: ic,
    s: {
      width: 16,
      height: 16
    }
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      minWidth: 0
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 8
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fw-medium) var(--fs-body) var(--font-sans)'
    }
  }, name), /*#__PURE__*/React.createElement(BRisk, {
    level: lvl
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fs-meta) var(--font-sans)',
      color: 'var(--text-muted)',
      marginTop: 2
    }
  }, desc)), /*#__PURE__*/React.createElement(BIconBtn, {
    icon: /*#__PURE__*/React.createElement(BIco, {
      n: "settings-2",
      s: {
        width: 15,
        height: 15
      }
    }),
    size: "sm",
    "aria-label": "Edit"
  }))))), /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement(BEye, {
    style: {
      marginBottom: 10
    }
  }, "Standing permissions (policy automation)"), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 7
    }
  }, [['Auto-summarize completed sessions to Brain', true], ['Auto-link branch names to plan tasks', true], ['Auto-create draft PR when checks pass', false], ['Auto-refresh stale owned docs', false]].map(([label, on], i) => /*#__PURE__*/React.createElement("div", {
    key: i,
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 10,
      padding: '9px 12px',
      borderRadius: 'var(--r-2)',
      border: '1px solid var(--border-subtle)',
      background: 'var(--surface-card)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fs-label) var(--font-sans)',
      color: 'var(--text-secondary)',
      flex: 1
    }
  }, label), /*#__PURE__*/React.createElement("span", {
    style: {
      width: 32,
      height: 18,
      borderRadius: 999,
      background: on ? 'var(--accent-solid)' : 'var(--surface-active)',
      position: 'relative',
      flex: 'none'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      position: 'absolute',
      top: 2,
      left: on ? 16 : 2,
      width: 14,
      height: 14,
      borderRadius: 999,
      background: on ? 'var(--accent-on-solid)' : 'var(--text-faint)',
      transition: 'left var(--dur-2)'
    }
  }))))), /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fs-micro)/1.5 var(--font-sans)',
      color: 'var(--text-faint)',
      marginTop: 8,
      display: 'flex',
      alignItems: 'center',
      gap: 6
    }
  }, /*#__PURE__*/React.createElement(BIco, {
    n: "info",
    s: {
      width: 13,
      height: 13
    }
  }), " Standing permissions are opt-in, narrow, revocable, and fully audited via the Action Gateway."))));
}
function UsageSection() {
  const u = KD5.usage;
  const max = Math.max(...u.spend14);
  const provColor = {
    claude: 'var(--domain-claude)',
    codex: 'var(--domain-codex)'
  };
  const card = {
    border: '1px solid var(--border-default)',
    borderRadius: 'var(--r-3)',
    background: 'var(--surface-card)',
    padding: '13px 14px'
  };
  return /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '16px',
      maxWidth: 820,
      display: 'flex',
      flexDirection: 'column',
      gap: 14
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      gridTemplateColumns: 'repeat(4, 1fr)',
      gap: 10
    }
  }, /*#__PURE__*/React.createElement(StatCard, {
    label: "Spend today",
    value: `$${u.today.spend}`,
    sub: `/ $${u.today.spendLimit} budget`,
    meter: u.today.spend / u.today.spendLimit,
    tone: "caution"
  }), /*#__PURE__*/React.createElement(StatCard, {
    label: "Tokens today",
    value: `${((u.today.tokensIn + u.today.tokensOut) / 1e6).toFixed(2)}M`,
    sub: `${u.today.tokensIn / 1e3 | 0}k in · ${u.today.tokensOut / 1e3 | 0}k out`
  }), /*#__PURE__*/React.createElement(StatCard, {
    label: "Aggregate context",
    value: `${u.today.context}k`,
    sub: `/ ${u.today.contextMax / 1000}M window`,
    meter: u.today.context / u.today.contextMax,
    tone: "accent"
  }), /*#__PURE__*/React.createElement(StatCard, {
    label: "Active sessions",
    value: u.today.sessions,
    sub: "across all projects"
  })), /*#__PURE__*/React.createElement("div", {
    style: card
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'baseline',
      gap: 8,
      marginBottom: 12
    }
  }, /*#__PURE__*/React.createElement(BEye, null, "Spend \xB7 last 14 days"), /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: 'auto',
      font: 'var(--fs-meta) var(--font-mono)',
      color: 'var(--text-muted)'
    }
  }, "$", u.spend14.reduce((a, b) => a + b, 0), " total")), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'flex-end',
      gap: 5,
      height: 96
    }
  }, u.spend14.map((v, i) => /*#__PURE__*/React.createElement("div", {
    key: i,
    title: `$${v}`,
    style: {
      flex: 1,
      height: `${v / max * 100}%`,
      borderRadius: '3px 3px 0 0',
      background: i === u.spend14.length - 1 ? 'var(--accent-solid)' : 'var(--accent-line)'
    }
  })))), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      gridTemplateColumns: '1fr 1fr',
      gap: 14
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: card
  }, /*#__PURE__*/React.createElement(BEye, {
    style: {
      marginBottom: 11
    }
  }, "Spend by profile"), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 9
    }
  }, u.byProfile.map((p, i) => /*#__PURE__*/React.createElement("div", {
    key: i
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      marginBottom: 4
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--font-mono)',
      color: provColor[p.provider],
      fontSize: 13
    }
  }, p.provider === 'claude' ? '✻' : '⌁'), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fs-label) var(--font-sans)',
      color: 'var(--text-primary)'
    }
  }, p.name), /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: 'auto',
      font: 'var(--fs-meta) var(--font-mono)',
      color: 'var(--text-secondary)'
    }
  }, "$", p.spend.toFixed(2))), /*#__PURE__*/React.createElement("div", {
    style: {
      height: 6,
      borderRadius: 999,
      background: 'var(--cap-track)',
      overflow: 'hidden'
    }
  }, /*#__PURE__*/React.createElement("i", {
    style: {
      display: 'block',
      height: '100%',
      width: p.pct + '%',
      background: provColor[p.provider]
    }
  })))))), /*#__PURE__*/React.createElement("div", {
    style: card
  }, /*#__PURE__*/React.createElement(BEye, {
    style: {
      marginBottom: 11
    }
  }, "Top context consumers"), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 10
    }
  }, u.topContext.map((c, i) => /*#__PURE__*/React.createElement("div", {
    key: i,
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 10
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1,
      font: 'var(--fs-label) var(--font-sans)',
      color: 'var(--text-secondary)',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap'
    }
  }, c.session), /*#__PURE__*/React.createElement(BMeter, {
    value: c.value,
    max: c.max,
    valueText: `${c.value}k`,
    style: {
      width: 150,
      flex: 'none'
    }
  })))))));
}
function StatCard({
  label,
  value,
  sub,
  meter,
  tone
}) {
  const fill = tone === 'caution' ? 'var(--cap-warn)' : tone === 'accent' ? 'var(--accent)' : 'var(--text-secondary)';
  return /*#__PURE__*/React.createElement("div", {
    style: {
      border: '1px solid var(--border-default)',
      borderRadius: 'var(--r-3)',
      background: 'var(--surface-card)',
      padding: '12px 13px'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fs-meta) var(--font-sans)',
      color: 'var(--text-muted)'
    }
  }, label), /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fw-semibold) var(--fs-h2)/1 var(--font-sans)',
      color: 'var(--text-primary)',
      margin: '6px 0 4px',
      fontVariantNumeric: 'tabular-nums'
    }
  }, value), /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fs-micro) var(--font-mono)',
      color: 'var(--text-faint)'
    }
  }, sub), meter != null && /*#__PURE__*/React.createElement("div", {
    style: {
      height: 4,
      borderRadius: 999,
      background: 'var(--cap-track)',
      overflow: 'hidden',
      marginTop: 8
    }
  }, /*#__PURE__*/React.createElement("i", {
    style: {
      display: 'block',
      height: '100%',
      width: Math.min(100, meter * 100) + '%',
      background: fill
    }
  })));
}
window.KitViews4 = {
  ProjectBrainPage,
  AuditTimeline,
  SettingsProfiles
};
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/control-plane/kit-views4.jsx", error: String((e && e.message) || e) }); }

// ui_kits/control-plane/kit-views5.jsx
try { (() => {
/* ============================================================
   Control Plane UI kit — Workflow Packs view.
   Enforces the pack-vs-instance distinction: a pack being
   installed never implies its commands are ready to run.
   ============================================================ */
const _NS6 = window.ControlPlaneDesignSystem_a21911;
const {
  Button: WBtn,
  IconButton: WIconBtn,
  StatusPill: WPill,
  RiskBadge: WRisk,
  MetaChip: WMeta
} = _NS6;
const WBadge = _NS6.Badge || (({
  children,
  mono,
  style = {}
}) => /*#__PURE__*/React.createElement("span", {
  style: {
    font: `var(--fw-medium) 11px/1 ${mono ? 'var(--font-mono)' : 'var(--font-sans)'}`,
    ...style
  }
}, children));
const {
  Ico: WIco,
  Eyebrow: WEye
} = window.KitShell;
const KD6 = window.KIT;
const {
  useState: useS6
} = React;

// instance status -> { pill, label, tone }
const INSTANCE = {
  active: {
    pill: 'running',
    label: 'Active'
  },
  ready: {
    pill: 'completed',
    label: 'Ready'
  },
  needs_personalization: {
    pill: 'waiting-perm',
    label: 'Needs personalization'
  },
  personalizing: {
    pill: 'running',
    label: 'Personalizing'
  },
  drift_detected: {
    pill: 'degraded',
    label: 'Drift detected'
  },
  upgrade_available: {
    pill: 'stale',
    label: 'Upgrade available'
  },
  detected: {
    pill: 'idle',
    label: 'Detected'
  },
  archived: {
    pill: 'archived',
    label: 'Archived'
  }
};
const PROVIDER = {
  bundled: 'Bundled',
  user: 'You',
  third_party: '3rd-party'
};
function WorkflowPacksView({
  openGateway,
  setView
}) {
  const [sel, setSel] = useS6('cc-crew');
  React.useEffect(() => {
    const t = setTimeout(() => window.lucide && window.lucide.createIcons(), 24);
    return () => clearTimeout(t);
  }, [sel]);
  const pack = KD6.packs.find(p => p.id === sel) || KD6.packs[0];
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      gridTemplateColumns: '300px 1fr',
      height: '100%',
      background: 'var(--surface-canvas)',
      minHeight: 0
    }
  }, /*#__PURE__*/React.createElement("aside", {
    style: {
      borderRight: '1px solid var(--border-default)',
      background: 'var(--surface-panel)',
      overflowY: 'auto'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '14px 14px 8px',
      display: 'flex',
      alignItems: 'center',
      gap: 8
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--teal-ink)'
    }
  }, /*#__PURE__*/React.createElement(WIco, {
    n: "package"
  })), /*#__PURE__*/React.createElement("h1", {
    style: {
      margin: 0,
      font: 'var(--fw-semibold) var(--fs-sub)/1 var(--font-sans)'
    }
  }, "Workflow Packs"), /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: 'auto'
    }
  }, /*#__PURE__*/React.createElement(WIconBtn, {
    icon: /*#__PURE__*/React.createElement(WIco, {
      n: "plus",
      s: {
        width: 15,
        height: 15
      }
    }),
    size: "sm",
    "aria-label": "Install pack"
  }))), /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '4px 8px 14px'
    }
  }, KD6.packs.map(p => {
    const inst = INSTANCE[p.instance] || INSTANCE.detected;
    const active = sel === p.id;
    return /*#__PURE__*/React.createElement("button", {
      key: p.id,
      onClick: () => setSel(p.id),
      style: {
        display: 'block',
        width: '100%',
        textAlign: 'left',
        cursor: 'pointer',
        border: 'none',
        borderRadius: 'var(--r-2)',
        padding: '9px 10px',
        marginBottom: 3,
        background: active ? 'var(--surface-active)' : 'transparent',
        boxShadow: active ? 'inset 0 0 0 1px var(--accent-line)' : 'none'
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 7
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        font: 'var(--fw-medium) var(--fs-body) var(--font-mono)',
        color: 'var(--text-primary)'
      }
    }, p.name), /*#__PURE__*/React.createElement("span", {
      style: {
        marginLeft: 'auto'
      }
    }, /*#__PURE__*/React.createElement(WPill, {
      status: inst.pill,
      size: "xs",
      label: inst.label,
      beacon: p.instance === 'needs_personalization'
    }))), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 6,
        marginTop: 6,
        font: 'var(--fs-micro) var(--font-mono)',
        color: 'var(--text-faint)'
      }
    }, /*#__PURE__*/React.createElement("span", null, "v", p.version), /*#__PURE__*/React.createElement("span", null, "\xB7"), /*#__PURE__*/React.createElement("span", null, PROVIDER[p.provider]), /*#__PURE__*/React.createElement("span", null, "\xB7"), /*#__PURE__*/React.createElement("span", null, p.project)));
  }))), /*#__PURE__*/React.createElement(PackDetail, {
    pack: pack,
    openGateway: openGateway,
    setView: setView
  }));
}
function PackDetail({
  pack,
  openGateway,
  setView
}) {
  const inst = INSTANCE[pack.instance] || INSTANCE.detected;
  const notReady = pack.instance === 'needs_personalization';
  const upgrade = pack.instance === 'upgrade_available';
  return /*#__PURE__*/React.createElement("div", {
    style: {
      overflowY: 'auto',
      minWidth: 0
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '16px',
      borderBottom: '1px solid var(--border-subtle)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 10,
      marginBottom: 8
    }
  }, /*#__PURE__*/React.createElement("h1", {
    style: {
      margin: 0,
      font: 'var(--fw-semibold) var(--fs-h2)/1 var(--font-mono)',
      letterSpacing: 'var(--tracking-tight)'
    }
  }, pack.name), /*#__PURE__*/React.createElement(WPill, {
    status: inst.pill,
    label: inst.label,
    beacon: notReady
  }), /*#__PURE__*/React.createElement(WBadge, {
    mono: true,
    style: {
      color: 'var(--text-muted)'
    }
  }, "v", pack.version), /*#__PURE__*/React.createElement("div", {
    style: {
      marginLeft: 'auto',
      display: 'flex',
      gap: 6
    }
  }, notReady && /*#__PURE__*/React.createElement(WBtn, {
    variant: "attention",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(WIco, {
      n: "wand-sparkles",
      s: {
        width: 14,
        height: 14
      }
    }),
    onClick: () => openGateway('edit')
  }, "Personalize"), upgrade && /*#__PURE__*/React.createElement(WBtn, {
    variant: "primary",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(WIco, {
      n: "arrow-up-circle",
      s: {
        width: 14,
        height: 14
      }
    }),
    onClick: () => openGateway('edit')
  }, "Upgrade"), !notReady && !upgrade && /*#__PURE__*/React.createElement(WBtn, {
    variant: "secondary",
    size: "sm",
    icon: /*#__PURE__*/React.createElement(WIco, {
      n: "settings-2",
      s: {
        width: 14,
        height: 14
      }
    })
  }, "Configure"))), /*#__PURE__*/React.createElement("p", {
    style: {
      margin: 0,
      font: 'var(--fs-body)/1.5 var(--font-sans)',
      color: 'var(--text-secondary)',
      maxWidth: 620
    }
  }, pack.desc)), notReady && /*#__PURE__*/React.createElement("div", {
    style: {
      margin: '14px 16px 0',
      display: 'flex',
      alignItems: 'flex-start',
      gap: 10,
      padding: '11px 13px',
      borderRadius: 'var(--r-3)',
      background: 'var(--caution-surface)',
      border: '1px solid var(--caution-line)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--caution-ink)',
      marginTop: 1
    }
  }, /*#__PURE__*/React.createElement(WIco, {
    n: "triangle-alert",
    s: {
      width: 16,
      height: 16
    }
  })), /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fw-semibold) var(--fs-label) var(--font-sans)',
      color: 'var(--caution-ink)'
    }
  }, "Template pack \u2014 not ready to run"), /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fs-meta)/1.5 var(--font-sans)',
      color: 'var(--text-secondary)',
      marginTop: 3
    }
  }, "This pack is installed but has no personalized instance for ", pack.project, ". Commands stay locked until personalization completes (a Gateway-reviewed run)."))), upgrade && /*#__PURE__*/React.createElement("div", {
    style: {
      margin: '14px 16px 0',
      display: 'flex',
      alignItems: 'center',
      gap: 10,
      padding: '11px 13px',
      borderRadius: 'var(--r-3)',
      background: 'var(--warning-surface)',
      border: '1px solid var(--warning-line)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--warning-ink)'
    }
  }, /*#__PURE__*/React.createElement(WIco, {
    n: "arrow-up-circle",
    s: {
      width: 16,
      height: 16
    }
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      font: 'var(--fs-meta)/1.5 var(--font-sans)',
      color: 'var(--text-secondary)'
    }
  }, /*#__PURE__*/React.createElement("strong", {
    style: {
      color: 'var(--warning-ink)'
    }
  }, "v", pack.version, " \u2192 newer available."), " Review the changelog before upgrading; owned files will be re-generated through the Gateway.")), /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '16px',
      display: 'grid',
      gridTemplateColumns: '1fr 240px',
      gap: 16,
      alignItems: 'start'
    }
  }, /*#__PURE__*/React.createElement("div", null, /*#__PURE__*/React.createElement(WEye, {
    style: {
      marginBottom: 10
    }
  }, "Commands \xB7 ", pack.commands.length), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 7
    }
  }, pack.commands.map((c, i) => {
    const locked = c.needsInstance && notReady;
    return /*#__PURE__*/React.createElement("div", {
      key: i,
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 10,
        padding: '10px 12px',
        borderRadius: 'var(--r-2)',
        border: '1px solid var(--border-default)',
        background: 'var(--surface-card)',
        opacity: locked ? 0.6 : 1
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        color: locked ? 'var(--text-faint)' : 'var(--accent-ink)'
      }
    }, /*#__PURE__*/React.createElement(WIco, {
      n: c.type === 'recipe' ? 'workflow' : 'slash',
      s: {
        width: 15,
        height: 15
      }
    })), /*#__PURE__*/React.createElement("code", {
      style: {
        font: 'var(--fw-medium) var(--fs-body) var(--font-mono)',
        color: 'var(--text-primary)'
      }
    }, c.name), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        gap: 5,
        marginLeft: 4
      }
    }, /*#__PURE__*/React.createElement(WBadge, {
      style: {
        display: 'inline-flex',
        alignItems: 'center',
        height: 16,
        padding: '0 6px',
        borderRadius: 'var(--r-1)',
        background: 'var(--neutral-surface)',
        color: 'var(--text-muted)'
      }
    }, c.type.replace('_', ' ')), c.creates && /*#__PURE__*/React.createElement(WBadge, {
      style: {
        display: 'inline-flex',
        alignItems: 'center',
        height: 16,
        padding: '0 6px',
        borderRadius: 'var(--r-1)',
        background: 'var(--teal-surface)',
        color: 'var(--teal-ink)'
      }
    }, "creates ", c.creates)), /*#__PURE__*/React.createElement("span", {
      style: {
        marginLeft: 'auto'
      }
    }, locked ? /*#__PURE__*/React.createElement("span", {
      style: {
        display: 'inline-flex',
        alignItems: 'center',
        gap: 5,
        font: 'var(--fs-meta) var(--font-sans)',
        color: 'var(--caution-ink)'
      }
    }, /*#__PURE__*/React.createElement(WIco, {
      n: "lock",
      s: {
        width: 12,
        height: 12
      }
    }), " needs instance") : /*#__PURE__*/React.createElement(WBtn, {
      variant: "ghost",
      size: "xs",
      icon: /*#__PURE__*/React.createElement(WIco, {
        n: "play",
        s: {
          width: 12,
          height: 12
        }
      }),
      onClick: () => c.creates === 'agent team' ? setView('team') : openGateway('q1')
    }, "Run")));
  })), pack.roles.length > 0 && /*#__PURE__*/React.createElement("div", {
    style: {
      marginTop: 18
    }
  }, /*#__PURE__*/React.createElement(WEye, {
    style: {
      marginBottom: 10
    }
  }, "Agent roles"), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      gap: 7,
      flexWrap: 'wrap'
    }
  }, pack.roles.map((r, i) => /*#__PURE__*/React.createElement("span", {
    key: i,
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 6,
      height: 26,
      padding: '0 10px',
      borderRadius: 'var(--r-2)',
      border: '1px solid var(--teal-line)',
      background: 'var(--teal-surface)',
      color: 'var(--teal-ink)',
      font: 'var(--fw-medium) var(--fs-label) var(--font-sans)'
    }
  }, /*#__PURE__*/React.createElement(WIco, {
    n: i === 0 ? 'workflow' : 'bot',
    s: {
      width: 13,
      height: 13
    }
  }), " ", r))))), /*#__PURE__*/React.createElement("div", {
    style: {
      border: '1px solid var(--border-subtle)',
      borderRadius: 'var(--r-3)',
      background: 'var(--surface-card)',
      padding: '13px 14px'
    }
  }, /*#__PURE__*/React.createElement(WEye, {
    style: {
      marginBottom: 11
    }
  }, "Capabilities"), /*#__PURE__*/React.createElement(CapRow, {
    icon: "terminal",
    label: "Commands",
    val: pack.commands.length
  }), /*#__PURE__*/React.createElement(CapRow, {
    icon: "users-round",
    label: "Agent roles",
    val: pack.roles.length || '—'
  }), /*#__PURE__*/React.createElement(CapRow, {
    icon: "rocket",
    label: "Launch recipes",
    val: pack.recipes
  }), /*#__PURE__*/React.createElement(CapRow, {
    icon: "file-text",
    label: "Plan parser",
    val: pack.parser || '—',
    mono: !!pack.parser
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      height: 1,
      background: 'var(--border-subtle)',
      margin: '10px 0'
    }
  }), /*#__PURE__*/React.createElement(CapRow, {
    icon: "git-merge",
    label: "Mutations",
    val: "via Gateway"
  }), /*#__PURE__*/React.createElement(CapRow, {
    icon: "badge-check",
    label: "Provider",
    val: PROVIDER[pack.provider]
  }))));
}
function CapRow({
  icon,
  label,
  val,
  mono
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      padding: '5px 0'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-faint)'
    }
  }, /*#__PURE__*/React.createElement(WIco, {
    n: icon,
    s: {
      width: 14,
      height: 14
    }
  })), /*#__PURE__*/React.createElement("span", {
    style: {
      font: 'var(--fs-label) var(--font-sans)',
      color: 'var(--text-muted)'
    }
  }, label), /*#__PURE__*/React.createElement("span", {
    style: {
      marginLeft: 'auto',
      font: `var(--fw-medium) var(--fs-label) ${mono ? 'var(--font-mono)' : 'var(--font-sans)'}`,
      color: 'var(--text-primary)'
    }
  }, val));
}
window.KitViews5 = {
  WorkflowPacksView
};
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/control-plane/kit-views5.jsx", error: String((e && e.message) || e) }); }

__ds_ns.Badge = __ds_scope.Badge;

__ds_ns.HarnessBadge = __ds_scope.HarnessBadge;

__ds_ns.MetaChip = __ds_scope.MetaChip;

__ds_ns.ProfileBadge = __ds_scope.ProfileBadge;

__ds_ns.Button = __ds_scope.Button;

__ds_ns.IconButton = __ds_scope.IconButton;

__ds_ns.DiffHunk = __ds_scope.DiffHunk;

__ds_ns.EvidenceChip = __ds_scope.EvidenceChip;

__ds_ns.GraphNode = __ds_scope.GraphNode;

__ds_ns.SessionRow = __ds_scope.SessionRow;

__ds_ns.AttentionMarker = __ds_scope.AttentionMarker;

__ds_ns.RiskBadge = __ds_scope.RiskBadge;

__ds_ns.STATUS = __ds_scope.STATUS;

__ds_ns.StatusPill = __ds_scope.StatusPill;

__ds_ns.UsageMeter = __ds_scope.UsageMeter;

})();
