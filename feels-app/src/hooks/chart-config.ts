// Chart configuration, styling, and constants for KLineChart integration

export type AxisType = 'normal' | 'logarithm' | 'percentage';
export type StylesLike = Record<string, any>;

// Chart identifiers and visual styling
export const PLOT_BACKGROUND = '#f8f8f8';

export const CHART_COLORS = {
  up: '#5cca39',        // success-500 - brand green for up candles
  down: '#ef4444',      // danger-500 - brand red for down candles  
  neutral: '#666666',
  priceMark: '#a6a6a6',
  lastPrice: '#b0b0b0',
  text: '#4B4B4B',
  grid: 'rgba(150, 150, 150, 0.15)',
  axisLine: 'rgba(128, 128, 128, 0.3)',
  crosshair: '#555555',
};

// Font stacks for canvas rendering (CSS variables don't work in canvas context)
export const FONT_DEFAULT = 'Terminal Grotesque, Helvetica, ui-sans-serif, system-ui, sans-serif';
export const FONT_MONO = 'JetBrains Mono, ui-monospace, monospace';

// Text styling from most recent commit
export const TEXT_STYLE_BASE = {
  size: 16,
  family: FONT_DEFAULT,
  weight: 'normal',
  color: '#ffffff',
};

export const TEXT_PADDING = {
  paddingLeft: 3.5,
  paddingRight: 3.5,
  paddingTop: 3.5,
  paddingBottom: 3.5,
};

export const TEXT_BOX_STYLE = {
  ...TEXT_STYLE_BASE,
  ...TEXT_PADDING,
  borderRadius: 2,
  borderSize: 1,
};

// Axis name mapping for KLineChart API
export const AXIS_NAME: Record<AxisType, AxisType> = {
  normal: 'normal',
  logarithm: 'logarithm',
  percentage: 'percentage',
};

/**
 * Build complete chart styles configuration
 */
export function buildChartStyles(params: {
  timezone: string;
  formatDate: (timestamp: number, format: string, type: number) => string;
  formatBigNumber: (value: string | number) => string;
  showUSD: boolean;
  usdConversionFactor: number;
}): StylesLike {
  const { formatBigNumber, showUSD, usdConversionFactor } = params;

  return {
    plotArea: {
      backgroundColor: PLOT_BACKGROUND,
    },
    grid: {
      show: true,
      horizontal: {
        show: true,
        color: CHART_COLORS.grid,
        style: 'solid',
        dashValue: [2, 2],
      },
      vertical: {
        show: false,
      },
    },
    candle: {
      type: 'candle_solid',
      bar: {
        upColor: CHART_COLORS.up,
        downColor: CHART_COLORS.down,
        noChangeColor: CHART_COLORS.neutral,
        upBorderColor: CHART_COLORS.up,
        downBorderColor: CHART_COLORS.down,
        noChangeBorderColor: CHART_COLORS.neutral,
        upWickColor: CHART_COLORS.up,
        downWickColor: CHART_COLORS.down,
        noChangeWickColor: CHART_COLORS.neutral,
      },
      tooltip: {
        showRule: 'always',
        showType: 'standard',
        labels: ['O: ', 'C: ', 'H: ', 'L: ', 'V: '],
        title: {
          show: false,
        },
        text: {
          size: 13,
          family: FONT_DEFAULT,
          weight: 'normal',
          color: CHART_COLORS.text,
          marginLeft: 8,
          marginTop: 4,
          marginRight: 8,
          marginBottom: 4,
        },
        values: (kLineData: any) => {
          const o = showUSD && usdConversionFactor > 0
            ? (kLineData.open ?? 0) * usdConversionFactor
            : (kLineData.open ?? 0);
          const c = showUSD && usdConversionFactor > 0
            ? (kLineData.close ?? 0) * usdConversionFactor
            : (kLineData.close ?? 0);
          const h = showUSD && usdConversionFactor > 0
            ? (kLineData.high ?? 0) * usdConversionFactor
            : (kLineData.high ?? 0);
          const l = showUSD && usdConversionFactor > 0
            ? (kLineData.low ?? 0) * usdConversionFactor
            : (kLineData.low ?? 0);
          const v = kLineData.volume ?? 0;

          return [
            formatBigNumber(o),
            formatBigNumber(c),
            formatBigNumber(h),
            formatBigNumber(l),
            formatBigNumber(v),
          ];
        },
      },
      priceMark: {
        show: true,
        high: {
          show: true,
          color: CHART_COLORS.priceMark,
          textMargin: 5,
          textSize: 12,
          textFamily: FONT_MONO,
          textWeight: 'normal',
        },
        low: {
          show: true,
          color: CHART_COLORS.priceMark,
          textMargin: 5,
          textSize: 12,
          textFamily: FONT_MONO,
          textWeight: 'normal',
        },
        last: {
          show: true,
          upColor: CHART_COLORS.lastPrice,
          downColor: CHART_COLORS.lastPrice,
          noChangeColor: CHART_COLORS.lastPrice,
          line: {
            show: true,
            style: 'dashed',
            dashValue: [4, 4],
            size: 1,
          },
          text: {
            ...TEXT_BOX_STYLE,
            show: true,
            style: 'fill',
            size: 13,
            paddingLeft: 5,
            paddingRight: 5,
            borderColor: CHART_COLORS.lastPrice,
            backgroundColor: CHART_COLORS.lastPrice,
          },
        },
      },
    },
    indicator: {
      lastValueMark: {
        show: false,
      },
      tooltip: {
        showRule: 'always',
        showType: 'standard',
        text: {
          size: 13,
          family: FONT_DEFAULT,
          weight: 'normal',
          color: CHART_COLORS.text,
        },
      },
    },
    xAxis: {
      show: true,
      axisLine: {
        show: true,
        color: CHART_COLORS.axisLine,
        size: 1,
      },
      tickLine: {
        show: true,
        length: 4,
        color: CHART_COLORS.axisLine,
        size: 1,
      },
      tickText: {
        show: true,
        color: CHART_COLORS.text,
        family: FONT_DEFAULT,
        weight: 'normal',
        size: 13,
        marginStart: 4,
        marginEnd: 4,
      },
    },
    yAxis: {
      show: true,
      position: 'right',
      type: 'normal',
      inside: false,
      reverse: false,
      axisLine: {
        show: true,
        color: CHART_COLORS.axisLine,
        size: 1,
      },
      tickLine: {
        show: true,
        length: 4,
        color: CHART_COLORS.axisLine,
        size: 1,
      },
      tickText: {
        show: true,
        color: CHART_COLORS.text,
        family: FONT_DEFAULT,
        weight: 'normal',
        size: 13,
        marginStart: 4,
        marginEnd: 4,
      },
    },
    crosshair: {
      show: true,
      horizontal: {
        show: true,
        line: {
          show: true,
          style: 'dashed',
          dashValue: [4, 2],
          size: 1,
          color: CHART_COLORS.crosshair,
        },
        text: {
          ...TEXT_BOX_STYLE,
          show: true,
          style: 'fill',
          size: 13,
          paddingLeft: 5,
          paddingRight: 5,
          borderColor: CHART_COLORS.crosshair,
          backgroundColor: CHART_COLORS.crosshair,
        },
      },
      vertical: {
        show: true,
        line: {
          show: true,
          style: 'dashed',
          dashValue: [4, 2],
          size: 1,
          color: CHART_COLORS.crosshair,
        },
        text: {
          ...TEXT_BOX_STYLE,
          show: true,
          style: 'fill',
          size: 13,
          paddingLeft: 5,
          paddingRight: 5,
          borderColor: CHART_COLORS.crosshair,
          backgroundColor: CHART_COLORS.crosshair,
        },
      },
    },
    overlay: {
      point: {
        color: '#1677FF',
        borderColor: 'rgba(22, 119, 255, 0.35)',
        borderSize: 1,
        radius: 5,
        activeColor: '#1677FF',
        activeBorderColor: 'rgba(22, 119, 255, 0.35)',
        activeBorderSize: 3,
        activeRadius: 5,
      },
      line: {
        style: 'solid',
        smooth: false,
        color: '#1677FF',
        size: 1,
        dashedValue: [2, 2],
      },
    },
  };
}

/**
 * Create floor price indicator configuration
 */
export function createFloorIndicator(floorData: Map<number, number>) {
  return {
    name: 'floor_line',
    figures: [
      {
        key: 'floor',
        title: 'Floor: ',
        type: 'line',
        baseValue: 0,
        styles: (data: any, _indicator: any, defaultStyles: any) => {
          const kLineData = data.kLineData as any;
          const floorValue = floorData.get(kLineData?.timestamp);
          if (floorValue !== undefined && kLineData) {
            return { color: 'rgba(255, 152, 0, 0.8)' };
          }
          return defaultStyles;
        },
      },
    ],
    calc: (kLineDataList: any[]) => {
      return kLineDataList.map((kLineData: any) => {
        const floorValue = floorData.get(kLineData.timestamp);
        if (floorValue !== undefined) {
          return { floor: floorValue, timestamp: kLineData.timestamp };
        }
        return {};
      });
    },
    draw: ({
      ctx,
      visibleRange,
      indicator,
      xAxis,
      yAxis,
    }: any) => {
      const { from, to } = visibleRange;
      ctx.lineWidth = 2;
      ctx.strokeStyle = 'rgba(255, 152, 0, 0.8)';
      ctx.beginPath();

      let started = false;
      for (let i = from; i < to; i++) {
        const data = indicator.result[i];
        if (data && data.floor !== undefined) {
          const x = xAxis.convertToPixel(i);
          const y = yAxis.convertToPixel(data.floor);
          if (!started) {
            ctx.moveTo(x, y);
            started = true;
          } else {
            ctx.lineTo(x, y);
          }
        }
      }
      ctx.stroke();
    },
  };
}

/**
 * Create GTWAP indicator configuration
 */
export function createGtwapIndicator(gtwapData: Map<number, number>) {
  return {
    name: 'gtwap_line',
    figures: [
      {
        key: 'gtwap',
        title: 'GTWAP: ',
        type: 'line',
        baseValue: 0,
        styles: (data: any, _indicator: any, defaultStyles: any) => {
          const kLineData = data.kLineData as any;
          const gtwapValue = gtwapData.get(kLineData?.timestamp);
          if (gtwapValue !== undefined && kLineData) {
            return { color: 'rgba(76, 175, 80, 0.8)' };
          }
          return defaultStyles;
        },
      },
    ],
    calc: (kLineDataList: any[]) => {
      return kLineDataList.map((kLineData: any) => {
        const gtwapValue = gtwapData.get(kLineData.timestamp);
        if (gtwapValue !== undefined) {
          return { gtwap: gtwapValue, timestamp: kLineData.timestamp };
        }
        return {};
      });
    },
    draw: ({
      ctx,
      visibleRange,
      indicator,
      xAxis,
      yAxis,
    }: any) => {
      const { from, to } = visibleRange;
      ctx.lineWidth = 2;
      ctx.strokeStyle = 'rgba(76, 175, 80, 0.8)';
      ctx.beginPath();

      let started = false;
      for (let i = from; i < to; i++) {
        const data = indicator.result[i];
        if (data && data.gtwap !== undefined) {
          const x = xAxis.convertToPixel(i);
          const y = yAxis.convertToPixel(data.gtwap);
          if (!started) {
            ctx.moveTo(x, y);
            started = true;
          } else {
            ctx.lineTo(x, y);
          }
        }
      }
      ctx.stroke();
    },
  };
}

