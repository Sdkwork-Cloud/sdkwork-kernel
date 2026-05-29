import type { MetricStripProps } from '../types/common-ui.types';

export function MetricStrip({ items }: MetricStripProps) {
  return (
    <dl className="metric-strip">
      {items.map((item) => (
        <div className="metric-strip__item" key={item.label}>
          <dt>{item.label}</dt>
          <dd className={item.tone ? `metric-strip__value metric-strip__value--${item.tone}` : 'metric-strip__value'}>
            {item.value}
          </dd>
        </div>
      ))}
    </dl>
  );
}
