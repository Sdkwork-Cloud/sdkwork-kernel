(function() {
  var style = getComputedStyle(document.documentElement);
  var accent = style.getPropertyValue('--accent').trim();
  var accent2 = style.getPropertyValue('--accent2').trim();
  var ink = style.getPropertyValue('--ink').trim();
  var muted = style.getPropertyValue('--muted').trim();
  var rule = style.getPropertyValue('--rule').trim();
  var bg2 = style.getPropertyValue('--bg2').trim();
  var critical = style.getPropertyValue('--sev-critical').trim();
  var high = style.getPropertyValue('--sev-high').trim();
  var medium = style.getPropertyValue('--sev-medium').trim();
  var low = style.getPropertyValue('--sev-low').trim();

  // --- Chart 1: Severity Distribution ---
  var chart1 = echarts.init(document.getElementById('chart-severity'), null, { renderer: 'svg' });
  chart1.setOption({
    animation: false,
    tooltip: { trigger: 'item', appendToBody: true },
    legend: { bottom: 0, textStyle: { color: muted, fontSize: 12 } },
    series: [{
      type: 'pie',
      radius: ['40%', '70%'],
      center: ['50%', '45%'],
      label: { color: ink, fontSize: 13, formatter: '{b}\n{c} 个' },
      labelLine: { lineStyle: { color: rule } },
      data: [
        { value: 5, name: 'Critical', itemStyle: { color: critical } },
        { value: 11, name: 'High', itemStyle: { color: high } },
        { value: 16, name: 'Medium', itemStyle: { color: medium } },
        { value: 8, name: 'Low', itemStyle: { color: low } }
      ]
    }]
  });
  window.addEventListener('resize', function() { chart1.resize(); });

  // --- Chart 2: Issue Category Breakdown ---
  var chart2 = echarts.init(document.getElementById('chart-category'), null, { renderer: 'svg' });
  chart2.setOption({
    animation: false,
    tooltip: { trigger: 'axis', appendToBody: true, axisPointer: { type: 'shadow' } },
    grid: { left: '3%', right: '4%', bottom: '3%', containLabel: true },
    xAxis: { type: 'value', axisLabel: { color: muted }, splitLine: { lineStyle: { color: rule } } },
    yAxis: { type: 'category', data: ['文档', '设计', '高可用', '安全', '性能', '功能'], axisLabel: { color: ink, fontSize: 12 }, axisLine: { lineStyle: { color: rule } } },
    series: [
      { name: 'Critical', type: 'bar', stack: 'total', itemStyle: { color: critical }, data: [0, 1, 2, 1, 0, 1] },
      { name: 'High', type: 'bar', stack: 'total', itemStyle: { color: high }, data: [1, 2, 3, 2, 2, 1] },
      { name: 'Medium', type: 'bar', stack: 'total', itemStyle: { color: medium }, data: [2, 3, 3, 3, 4, 3] },
      { name: 'Low', type: 'bar', stack: 'total', itemStyle: { color: low }, data: [1, 1, 1, 2, 2, 1] }
    ],
    legend: { bottom: 0, textStyle: { color: muted, fontSize: 11 } }
  });
  window.addEventListener('resize', function() { chart2.resize(); });

  // --- Chart 3: Commercial Readiness Radar ---
  var chart3 = echarts.init(document.getElementById('chart-radar'), null, { renderer: 'svg' });
  chart3.setOption({
    animation: false,
    tooltip: { appendToBody: true },
    radar: {
      indicator: [
        { name: '多租户', max: 100 },
        { name: '计费计量', max: 100 },
        { name: '安全防护', max: 100 },
        { name: '高可用', max: 100 },
        { name: '可观测性', max: 100 },
        { name: 'UI/UX', max: 100 },
        { name: 'SDK生态', max: 100 },
        { name: '文档体系', max: 100 }
      ],
      axisName: { color: ink, fontSize: 12 },
      splitLine: { lineStyle: { color: rule } },
      splitArea: { areaStyle: { color: ['transparent', bg2] } },
      axisLine: { lineStyle: { color: rule } }
    },
    series: [{
      type: 'radar',
      data: [
        { value: [75, 20, 55, 45, 50, 15, 70, 80], name: '当前成熟度', itemStyle: { color: accent }, areaStyle: { color: accent + '33' } },
        { value: [90, 85, 90, 85, 85, 80, 85, 90], name: '商业化目标', itemStyle: { color: accent2 }, areaStyle: { color: accent2 + '22' }, lineStyle: { type: 'dashed' } }
      ]
    }],
    legend: { bottom: 0, textStyle: { color: muted, fontSize: 12 } }
  });
  window.addEventListener('resize', function() { chart3.resize(); });

  // --- Chart 4: Improvement Roadmap Gantt ---
  var chart4 = echarts.init(document.getElementById('chart-roadmap'), null, { renderer: 'svg' });
  var phases = ['P0: 紧急修复', 'P1: 核心加固', 'P2: 性能优化', 'P3: 商业化硬化', 'P4: 生态完善'];
  var phaseColors = [critical, high, medium, accent, low];
  chart4.setOption({
    animation: false,
    tooltip: { appendToBody: true, formatter: function(p) { return p.name + '<br/>周期: ' + p.value[1] + ' - ' + p.value[2] + ' 周'; } },
    grid: { left: '3%', right: '4%', bottom: '3%', containLabel: true },
    xAxis: { type: 'value', name: '周', axisLabel: { color: muted }, splitLine: { lineStyle: { color: rule } }, nameTextStyle: { color: muted } },
    yAxis: { type: 'category', data: phases, axisLabel: { color: ink, fontSize: 12 }, axisLine: { lineStyle: { color: rule } } },
    series: [{
      type: 'custom',
      renderItem: function(params, api) {
        var cat = api.value(0);
        var start = api.coord([api.value(1), cat]);
        var end = api.coord([api.value(2), cat]);
        var height = api.size([0, 1])[1] * 0.5;
        return {
          type: 'rect',
          shape: { x: start[0], y: start[1] - height / 2, width: end[0] - start[0], height: height },
          style: { fill: phaseColors[cat], opacity: 0.85 }
        };
      },
      encode: { x: [1, 2], y: 0 },
      data: [
        [0, 0, 2],
        [1, 1, 4],
        [2, 3, 7],
        [3, 6, 12],
        [4, 10, 16]
      ]
    }]
  });
  window.addEventListener('resize', function() { chart4.resize(); });
})();
