// Renders the configuration controls for the PriceChart component.
import React from 'react';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { ChevronDown } from 'lucide-react';

// ========================================
// Type Definitions
// ========================================

// Timezone configuration with GMT offset for display
interface TimezoneOption {
  value: string;
  label: string;
  offset: number;
}

// Component props for all toolbar controls
interface PriceChartToolbarProps {
  // Time range selection
  timeRange: string;
  onTimeRangeChange: (range: string) => void;
  
  // Timezone configuration
  timezone: string;
  onTimezoneChange: (zone: string) => void;
  currentTime: Date;
  timezones: TimezoneOption[];
  
  // Chart overlay toggles
  showUSD: boolean;
  onToggleUSD: () => void;
  showFloorPrice: boolean;
  onToggleFloor: () => void;
  showGTWAPPrice: boolean;
  onToggleGTWAP: () => void;
  showLastPrice: boolean;
  onToggleLastPrice: () => void;
  
  // Price axis configuration
  priceAxisType: 'normal' | 'logarithm' | 'percentage';
  onPriceAxisTypeChange: (type: 'normal' | 'logarithm' | 'percentage') => void;
}

// ========================================
// Constants
// ========================================

// Available time range options for chart display
const TIME_RANGE_VALUES: Array<'1m' | '1h' | '1D' | '1W' | '1M' | 'all'> = [
  '1m',
  '1h',
  '1D',
  '1W',
  '1M',
  'all',
];

// ========================================
// Utility Functions
// ========================================

// Formats time for display in 24-hour format with timezone support
function formatTime(zone: string, currentTime: Date) {
  try {
    return new Intl.DateTimeFormat('en-US', {
      timeZone: zone,
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      hour12: false,
    }).format(currentTime);
  } catch (e) {
    return '';
  }
}

// Retrieves human-readable timezone label with fallback parsing
function getTimezoneLabel(timezone: string, options: TimezoneOption[]) {
  const zone = options.find((z) => z.value === timezone);
  if (zone) return zone.label;
  // Fallback: extract city name from timezone string
  const parts = timezone.split('/');
  return parts[parts.length - 1]?.replace(/_/g, ' ') || 'Unknown';
}

// ========================================
// Main Component
// ========================================

export function PriceChartToolbar({
  timeRange,
  onTimeRangeChange,
  timezone,
  onTimezoneChange,
  currentTime,
  timezones,
  showUSD,
  onToggleUSD,
  showFloorPrice,
  onToggleFloor,
  showGTWAPPrice,
  onToggleGTWAP,
  showLastPrice,
  onToggleLastPrice,
  priceAxisType,
  onPriceAxisTypeChange,
}: PriceChartToolbarProps) {
  return (
    <div className="flex flex-col gap-3">
      {/* Horizontal layout with chart overlays on left and time controls on right */}
      <div className="flex items-center justify-between">
        {/* Left section: Chart overlay toggles for price lines and currency */}
        <div className="flex items-center gap-4 text-xs pl-2">
          <ToggleButton label="USD" active={showUSD} onClick={onToggleUSD} activeClass="text-black" />
          <ToggleButton
            label="Floor Price"
            active={showFloorPrice}
            onClick={onToggleFloor}
            activeClass="text-[#3B82F6]"
          />
          <ToggleButton
            label="GTWAP"
            active={showGTWAPPrice}
            onClick={onToggleGTWAP}
            activeClass="text-success-500"
          />
          <ToggleButton
            label="Last Price"
            active={showLastPrice}
            onClick={onToggleLastPrice}
            activeClass="text-[#a6a6a6]"
            inactiveClass="text-[#a6a6a6]/60 hover:text-[#a6a6a6]"
          />
        </div>

        {/* Right section: Time and display controls */}
        <div className="flex flex-wrap items-center gap-2">
          {/* Time range selector buttons (1m, 1h, 1D, etc.) */}
          <div className="flex items-center gap-1">
            {TIME_RANGE_VALUES.map((range) => (
              <Button
                key={range}
                variant={timeRange === range ? 'default' : 'outline'}
                size="sm"
                onClick={() => onTimeRangeChange(range)}
                className="h-7 px-2 py-0.5 text-xs font-bold min-w-[32px] border-border hover:border-success-500"
              >
                {range === 'all' ? 'All' : range}
              </Button>
            ))}
          </div>

          {/* Price axis type selector (Linear, Log, Percentage) */}
          <div className="ml-2">
            <AxisDropdown priceAxisType={priceAxisType} onPriceAxisTypeChange={onPriceAxisTypeChange} />
          </div>

          {/* Timezone selector with live time display */}
          <TimezoneDropdown
            timezone={timezone}
            onTimezoneChange={onTimezoneChange}
            currentTime={currentTime}
            options={timezones}
          />
        </div>
      </div>
    </div>
  );
}

// ========================================
// Toggle Button Component
// ========================================

// Radio-style toggle button for chart overlay options
function ToggleButton({
  label,
  active,
  onClick,
  activeClass,
  inactiveClass,
}: {
  label: string;
  active: boolean;
  onClick: () => void;
  activeClass: string;
  inactiveClass?: string;
}) {
  return (
    <button
      onClick={onClick}
      className={`flex items-center gap-1.5 cursor-pointer transition-colors ${
        active ? `${activeClass} font-medium` : (inactiveClass || 'text-muted-foreground hover:text-black')
      }`}
    >
      {/* Radio button indicator with dynamic fill based on active state */}
      <div className="relative h-3 w-3 rounded-full border border-current flex items-center justify-center">
        {active && <div className="h-1.5 w-1.5 rounded-full bg-current" />}
      </div>
      <span>{label}</span>
    </button>
  );
}

// ========================================
// Timezone Dropdown Component
// ========================================

// Dropdown selector for timezone with live time display
function TimezoneDropdown({
  timezone,
  onTimezoneChange,
  currentTime,
  options,
}: {
  timezone: string;
  onTimezoneChange: (zone: string) => void;
  currentTime: Date;
  options: TimezoneOption[];
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="outline"
          size="sm"
          className="h-7 px-2 text-xs font-normal w-48 flex items-center justify-between group border-border hover:text-white hover:border-success-500 focus-visible:ring-0 focus-visible:ring-offset-0"
        >
          <div className="flex items-center justify-between w-full">
            <div className="flex items-center gap-3">
              {/* GMT offset indicator with dynamic calculation */}
              <span className="text-[9px] text-black font-mono font-bold group-hover:text-white">
                GMT
                {(() => {
                  const offset = options.find((z) => z.value === timezone)?.offset || 0;
                  const sign = offset >= 0 ? '+' : '-';
                  const absOffset = Math.abs(offset).toString().padStart(2, '0');
                  return `${sign}${absOffset}`;
                })()}
              </span>
              <span className="group-hover:text-white">{getTimezoneLabel(timezone, options)}</span>
            </div>
            <div className="flex items-center gap-0.5 mr-0.5">
              {/* Real-time clock showing current time in selected timezone */}
              <span className="font-mono font-bold text-[9px] leading-none group-hover:text-white">
                {formatTime(timezone, currentTime)}
              </span>
              <ChevronDown className="h-3 w-3 group-hover:text-white" />
            </div>
          </div>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-48 max-h-96 overflow-y-auto">
        {/* Timezone options list with GMT offsets and live times */}
        {options.map((zone) => (
          <DropdownMenuItem
            key={zone.value}
            onClick={() => onTimezoneChange(zone.value)}
            className="text-xs hover:bg-primary hover:text-white [&>div_span]:hover:text-white"
          >
            <div className="flex items-center justify-between w-full">
              <div className="flex items-center gap-3">
                {/* GMT offset badge for each timezone option */}
                <span className="text-[9px] text-black font-mono font-bold">
                  GMT{zone.offset >= 0 ? '+' : '-'}
                  {Math.abs(zone.offset).toString().padStart(2, '0')}
                </span>
                <span>{zone.label}</span>
              </div>
              {/* Current time in this timezone for easy comparison */}
              <span className="font-mono font-bold text-[9px]">
                {formatTime(zone.value, currentTime)}
              </span>
            </div>
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

// ========================================
// Axis Type Dropdown Component
// ========================================

// Dropdown for selecting price axis scaling mode
function AxisDropdown({
  priceAxisType,
  onPriceAxisTypeChange,
}: {
  priceAxisType: 'normal' | 'logarithm' | 'percentage';
  onPriceAxisTypeChange: (type: 'normal' | 'logarithm' | 'percentage') => void;
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="outline"
          size="sm"
          className="h-7 px-2 text-xs font-normal w-12 flex items-center justify-between group border-border hover:text-white hover:border-success-500 focus-visible:ring-0 focus-visible:ring-offset-0"
        >
          {/* Abbreviated axis type labels (Lin/Log/%) */}
          <span className="text-left group-hover:text-white">
            {priceAxisType === 'normal'
              ? 'Lin'
              : priceAxisType === 'logarithm'
                ? 'Log'
                : '%'}
          </span>
          <ChevronDown className="h-3 w-3 ml-auto group-hover:text-white" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="min-w-0 w-12">
        {/* Three axis scaling options: Linear, Logarithmic, Percentage */}
        {['normal', 'logarithm', 'percentage'].map((type) => (
          <DropdownMenuItem
            key={type}
            onClick={() => onPriceAxisTypeChange(type as 'normal' | 'logarithm' | 'percentage')}
            className="text-xs hover:data-[highlighted]:bg-primary hover:data-[highlighted]:text-white"
          >
            <span className={priceAxisType === type ? 'font-medium' : ''}>
              {type === 'normal' ? 'Lin' : type === 'logarithm' ? 'Log' : '%'}
            </span>
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
