import { useEffect, useRef } from 'react';
import * as echarts from 'echarts/core';
import { LineChart, BarChart, PieChart } from 'echarts/charts';
import {
  GridComponent,
  TooltipComponent,
  LegendComponent,
  AriaComponent,
} from 'echarts/components';
import { SVGRenderer } from 'echarts/renderers';
import type { EChartsOption } from 'echarts';
import { useVisual } from '@/lib/appearance';
echarts.use([
  LineChart,
  BarChart,
  PieChart,
  GridComponent,
  TooltipComponent,
  LegendComponent,
  AriaComponent,
  SVGRenderer,
]);
export function Chart({
  option,
  label,
  onSelect,
}: {
  option: EChartsOption;
  label: string;
  onSelect?: (index: number) => void;
}) {
  const target = useRef<HTMLDivElement>(null);
  const chart = useRef<ReturnType<typeof echarts.init> | null>(null);
  const handler = useRef(onSelect);
  handler.current = onSelect;
  const visual = useVisual();
  useEffect(() => {
    if (!target.current) return;
    const instance = echarts.init(target.current, undefined, { renderer: 'svg' });
    chart.current = instance;
    const observer = new ResizeObserver(() => instance.resize());
    observer.observe(target.current);
    instance.on('click', (event) => handler.current?.(event.dataIndex));
    return () => {
      observer.disconnect();
      instance.dispose();
      chart.current = null;
    };
  }, []);
  useEffect(() => {
    chart.current?.setOption(
      {
        color: [
          visual.tokens['--chart-1'],
          visual.tokens['--chart-2'],
          visual.tokens['--chart-3'],
          visual.tokens['--chart-4'],
          visual.tokens['--chart-5'],
        ],
        textStyle: {
          fontFamily: 'Manrope Variable',
          fontSize: (11 * visual.appearance.config.fontScale) / 100,
          color: visual.tokens['--subtle'],
        },
        animation: visual.motion,
        animationDuration: visual.duration * 1000,
        animationDurationUpdate: visual.duration * 1000,
        aria: { enabled: true, label: { description: label } },
        ...option,
      },
      true,
    );
  }, [
    option,
    visual.tokens,
    visual.motion,
    visual.duration,
    visual.appearance.config.fontScale,
    label,
  ]);
  return <div className="chart" ref={target} role="img" aria-label={label} />;
}
