import { ArrowPathIcon } from "@heroicons/react/24/outline";
import type { ReactNode } from "react";

export enum IndicatorState {
  Working = "Working",
  Correct = "Correct",
  Mismatch = "Mismatch",
  TimeOut = "TimeOut",
  Error = "Error",
}

export const INDICATOR_TEXT_COLOR = {
  Working: "text-working",
  Correct: "text-correct",
  Mismatch: "text-mismatch",
  TimeOut: "text-time-out",
  Error: "text-error",
} satisfies Record<IndicatorState, string>;
export const INDICATOR_BG_COLOR = {
  Working: "bg-working",
  Correct: "bg-correct",
  Mismatch: "bg-mismatch",
  TimeOut: "bg-time-out",
  Error: "bg-error",
} satisfies Record<IndicatorState, string>;

const INDICATOR_SYMBOL = {
  Working: <ArrowPathIcon className="w-3 animate-spin" />,
  Correct: "C",
  Mismatch: "M",
  TimeOut: "T",
  Error: "E",
} satisfies Record<IndicatorState, ReactNode>;

export const Indicator = ({ state }: { state: IndicatorState }) => (
  <span
    className={
      "inline-grid aspect-square w-6 place-items-center rounded " +
      INDICATOR_BG_COLOR[state]
    }
  >
    <span className="font-mono text-sm text-white">
      {INDICATOR_SYMBOL[state]}
    </span>
  </span>
);
