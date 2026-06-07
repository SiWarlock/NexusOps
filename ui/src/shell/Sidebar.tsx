/**
 * Left sidebar region. 6.1b reserves the region; the attention-weighted nav
 * (status → attention-rank ordering driving sidebar weight) lands in 6.2.
 */
export function Sidebar() {
  return (
    <aside className="sidebar" aria-label="Sidebar" style={{ width: 240 }}>
      <nav aria-label="Primary" />
    </aside>
  );
}
