import type { ReactNode } from 'react';

export default function TerminalFrame({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="terminal">
      <div className="terminalbar">
        <div>
          <i />
          <i />
          <i />
        </div>
        <span>{title}</span>
      </div>
      <div className="terminalbody">{children}</div>
    </section>
  );
}
