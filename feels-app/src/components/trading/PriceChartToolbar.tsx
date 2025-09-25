// Renders the toolbar (toggles, selectors) for the PriceChart component.
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
// Types and Constants
// ========================================

interface TimezoneOption {
  value: string;
  label: string;
  offset: number;
}

interface PriceChartToolbarProps {
  timeRange: string;
  onTimeRangeChange: (range: string) => void;
  timezone: string;
  onTimezoneChange: (zone: string) => void;
  currentTime: Date;
  timezones: TimezoneOption[];
  showUSD: boolean;
  onToggleUSD: () => void;
  showFloorPrice: boolean;
  onToggleFloor: () => void;
  showGTWAPPrice: boolean;
  onToggleGTWAP: () => void;
  showLastPrice: boolean;
  onToggleLastPrice: () => void;
  priceAxisType: 'normal' | 'logarithm' | 'percentage';
  onPriceAxisTypeChange: (type: 'normal' | 'logarithm' | 'percentage') => void;
}

const TIME_RANGE_VALUES: Array<'1m' | '1h' | '1D' | '1W' | '1M' | 'all'> = ['1m', '1h', '1D', '1W', '1M', 'all'];

// ========================================
// Utility Functions
// ========================================

// Format time for display in timezone dropdown
function formatTime(zone: string, currentTime: Date) {
  try {
    return new Intl.DateTimeFormat('en-US', {
      timeZone: zone,
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      hour12: false
    }).format(currentTime);
  } catch (e) {
    return '';
  }
}

// Get display label for timezone
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
  onPriceAxisTypeChange
}: PriceChartToolbarProps) {
  return (
    <div className="flex flex-col gap-1 pr-6">
      {/* Toggle switches for chart overlays */}
      <div className="flex items-center justify-end gap-3 text-xs">
        <ToggleButton label="USD" active={showUSD} onClick={onToggleUSD} activeClass="text-black" />
        <ToggleButton label="Floor Price" active={showFloorPrice} onClick={onToggleFloor} activeClass="text-[#3B82F6]" />
        <ToggleButton label="GTWAP" active={showGTWAPPrice} onClick={onToggleGTWAP} activeClass="text-[#5cca39]" />
        <ToggleButton label="Last Price" active={showLastPrice} onClick={onToggleLastPrice} activeClass="text-gray-800" />
      </div>

      {/* Time range buttons and dropdowns */}
      <div className="flex items-center gap-2 mt-2">
        {/* Time range selector buttons */}
        <div className="flex items-center gap-1">
          {TIME_RANGE_VALUES.map((range) => (
            <Button
              key={range}
              variant={timeRange === range ? 'default' : 'outline'}
              size="sm"
              onClick={() => onTimeRangeChange(range)}
              className="h-6 px-0 py-0.5 text-xs font-bold w-8 border-border hover:border-green-500"
            >
              {range.toUpperCase()}
            </Button>
          ))}
        </div>

        {/* Timezone selector dropdown */}
        <TimezoneDropdown
          timezone={timezone}
          onTimezoneChange={onTimezoneChange}
          currentTime={currentTime}
          options={timezones}
        />

        {/* Price axis type dropdown */}
        <AxisDropdown
          priceAxisType={priceAxisType}
          onPriceAxisTypeChange={onPriceAxisTypeChange}
        />
      </div>
    </div>
  );
}

// ========================================
// Sub-components
// ========================================

// Toggle button with radio-style visual indicator
function ToggleButton({ label, active, onClick, activeClass }: { label: string; active: boolean; onClick: () => void; activeClass: string }) {
  return (
    <button
      onClick={onClick}
      className={`flex items-center gap-1.5 cursor-pointer transition-colors ${
        active ? `${activeClass} font-medium` : 'text-muted-foreground hover:text-black'
      }`}
    >
      {/* Radio-style indicator */}
      <div className="relative h-3 w-3 rounded-full border border-current flex items-center justify-center">
        {active && <div className="h-1.5 w-1.5 rounded-full bg-current" />}
      </div>
      <span>{label}</span>
    </button>
  );
}

// Timezone selector with live time display
function TimezoneDropdown({ timezone, onTimezoneChange, currentTime, options }: { timezone: string; onTimezoneChange: (zone: string) => void; currentTime: Date; options: TimezoneOption[] }) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" size="sm" className="h-6 px-2 text-xs font-normal w-48 flex items-center justify-between group border-border hover:text-white hover:border-green-500 ml-2">
          <div className="flex items-center justify-between w-full">
            <div className="flex items-center gap-2">
              {/* GMT offset display */}
              <span className="text-black font-mono group-hover:text-white">
                GMT{(() => {
                  const offset = options.find((z) => z.value === timezone)?.offset || 0;
                  return offset >= 0 ? `+${offset}` : offset.toString();
                })()}
              </span>
              <span className="group-hover:text-white">{getTimezoneLabel(timezone, options)}</span>
            </div>
            <div className="flex items-center gap-0.5 mr-0.5">
              {/* Live time in selected timezone */}
              <span className="font-['JetBrains_Mono'] text-[9px] leading-none group-hover:text-white">
                {formatTime(timezone, currentTime)}
              </span>
              <ChevronDown className="h-3 w-3 group-hover:text-white" />
            </div>
          </div>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-48 max-h-96 overflow-y-auto">
        {options.map((zone) => (
          <DropdownMenuItem
            key={zone.value}
            onClick={() => onTimezoneChange(zone.value)}
            className="text-xs hover:bg-primary hover:text-white [&>div_span]:hover:text-white"
          >
            <div className="flex items-center justify-between w-full">
              <div className="flex items-center gap-2">
                {/* GMT offset for each option */}
                <span className="text-xs text-black font-mono">GMT{zone.offset >= 0 ? '+' : ''}{zone.offset}</span>
                <span>{zone.label}</span>
              </div>
              {/* Live time in this timezone */}
              <span className="font-['JetBrains_Mono'] text-[9px]">{formatTime(zone.value, currentTime)}</span>
            </div>
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

// Price axis scaling type selector
function AxisDropdown({ priceAxisType, onPriceAxisTypeChange }: { priceAxisType: 'normal' | 'logarithm' | 'percentage'; onPriceAxisTypeChange: (type: 'normal' | 'logarithm' | 'percentage') => void }) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" size="sm" className="h-6 px-2 text-xs font-normal w-24 flex items-center justify-between group border-border hover:text-white hover:border-green-500 ml-2">
          {/* Display current axis type */}
          <span className="text-left group-hover:text-white">
            {priceAxisType === 'normal' ? 'Linear' : priceAxisType === 'logarithm' ? 'Logarithmic' : 'Percentage'}
          </span>
          <ChevronDown className="h-3 w-3 ml-auto group-hover:text-white" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="min-w-0 w-[95px]">
        {/* Axis scaling options */}
        {['normal', 'logarithm', 'percentage'].map((type) => (
          <DropdownMenuItem
            key={type}
            onClick={() => onPriceAxisTypeChange(type as 'normal' | 'logarithm' | 'percentage')}
            className="text-xs hover:data-[highlighted]:bg-primary hover:data-[highlighted]:text-white"
          >
            <span className={priceAxisType === type ? 'font-medium' : ''}>
              {type === 'normal' ? 'Linear' : type === 'logarithm' ? 'Logarithmic' : 'Percentage'}
            </span>
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}


