import type { ReactNode } from 'react';

export type StatusTone = 'good' | 'warn' | 'bad' | 'neutral';

export interface KernelPanelProps {
  title: string;
  eyebrow?: string;
  actions?: ReactNode;
  children: ReactNode;
}

export interface StatusBadgeProps {
  tone: StatusTone;
  children: ReactNode;
}

export interface MetricStripItem {
  label: string;
  value: string | number;
  tone?: StatusTone;
}

export interface MetricStripProps {
  items: MetricStripItem[];
}
