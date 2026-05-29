import type { KernelPanelProps } from '../types/common-ui.types';

export function KernelPanel({ title, eyebrow, actions, children }: KernelPanelProps) {
  return (
    <section className="kernel-panel">
      <header className="kernel-panel__header">
        <div>
          {eyebrow ? <p className="kernel-panel__eyebrow">{eyebrow}</p> : null}
          <h2>{title}</h2>
        </div>
        {actions ? <div className="kernel-panel__actions">{actions}</div> : null}
      </header>
      {children}
    </section>
  );
}
