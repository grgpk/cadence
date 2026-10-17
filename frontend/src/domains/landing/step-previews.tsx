import { Check, Link2 } from "lucide-react";

import { APP_NAME } from "../../lib/config";

const FRAME = "flex h-36 flex-col justify-center rounded-xl border bg-background p-4";

/** Open hours for a sample week, as percentages of the working day. */
const WEEK = [
  { day: "Mon", from: 12, to: 62 },
  { day: "Tue", from: 12, to: 88 },
  { day: "Wed", from: 38, to: 88 },
  { day: "Thu", from: 12, to: 62 },
  { day: "Fri", from: 12, to: 44 },
];

/** Step 1: a week with the hours someone takes calls. */
export function HoursPreview() {
  return (
    <div aria-hidden="true" className={`${FRAME} gap-2.5`}>
      {WEEK.map(({ day, from, to }) => (
        <div key={day} className="flex items-center gap-3">
          <span className="w-8 text-xs font-medium text-muted-foreground">{day}</span>
          <span className="relative h-2 flex-1 rounded-full bg-muted">
            <span
              className="absolute inset-y-0 rounded-full bg-noon"
              style={{ left: `${from}%`, width: `${to - from}%` }}
            />
          </span>
        </div>
      ))}
    </div>
  );
}

/** Step 2: the one link, just copied. */
export function LinkPreview() {
  return (
    <div aria-hidden="true" className={`${FRAME} gap-3`}>
      <div className="flex items-center gap-2 rounded-md border bg-card px-3 py-2 text-sm font-medium">
        <Link2 className="size-4 shrink-0 text-muted-foreground" />
        <span className="truncate">
          <span className="text-muted-foreground">{APP_NAME.toLowerCase()}/book/</span>
          ana
        </span>
      </div>
      <p className="flex items-center gap-1.5 text-sm font-medium text-success">
        <Check className="size-4" />
        Link copied
      </p>
    </div>
  );
}

const TIMES = [
  { time: "09:00", rail: "bg-dawn", picked: false },
  { time: "09:30", rail: "bg-dawn", picked: false },
  { time: "13:00", rail: "bg-noon", picked: true },
  { time: "17:30", rail: "bg-dusk", picked: false },
];

/** Step 3: times to choose from, one of them taken. */
export function TimesPreview() {
  return (
    <div aria-hidden="true" className={`${FRAME} `}>
      <div className="grid grid-cols-2 gap-2">
        {TIMES.map(({ time, rail, picked }) => (
          <span
            key={time}
            className="relative flex h-10 items-center justify-center overflow-hidden rounded-md border bg-card text-clock text-base font-medium"
          >
            <span
              className={`absolute inset-y-0 left-0 ${rail} ${picked ? "w-full" : "w-[3px]"}`}
            />
            <span className={`relative ${picked ? "text-day-foreground" : ""}`}>
              {time}
            </span>
          </span>
        ))}
      </div>
    </div>
  );
}
