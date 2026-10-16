import { cn } from "cn";
import { useEffect, useRef, useState } from "react";

import { HourMark } from "../shared/hour-mark";
import { dayPhase, type DayPhase } from "../../lib/zoned-time";

type Block =
  | { kind: "open" }
  | { kind: "booked"; name: string }
  /** Open at first, then someone books it while you watch. */
  | { kind: "booking"; name: string; delaySeconds: number };

/** One sample working day. `null` is an hour the host keeps free. */
const DAY: { hour: number; block: Block | null }[] = [
  { hour: 8, block: { kind: "open" } },
  { hour: 9, block: { kind: "booked", name: "Ana" } },
  { hour: 10, block: { kind: "booking", name: "Leo", delaySeconds: 0 } },
  { hour: 11, block: { kind: "open" } },
  { hour: 12, block: null },
  { hour: 13, block: { kind: "booked", name: "Mia" } },
  { hour: 14, block: { kind: "booking", name: "Sam", delaySeconds: 5 } },
  { hour: 15, block: { kind: "open" } },
  { hour: 16, block: { kind: "booked", name: "Noor" } },
  { hour: 17, block: { kind: "booking", name: "Ivo", delaySeconds: 10 } },
  { hour: 18, block: { kind: "open" } },
  { hour: 19, block: null },
];

export const OPEN_STYLES: Record<DayPhase, string> = {
  // Dark fills stay thin: thick warm washes turn brown on near-black.
  Morning: "border-dawn bg-dawn/35 dark:bg-dawn/10",
  Afternoon: "border-noon bg-noon/30 dark:bg-noon/10",
  Evening: "border-dusk/60 bg-dusk/15 dark:bg-dusk/20",
};

const PAUSE_ON_HOVER = "group-hover:[animation-play-state:paused]";

function TapeBlock({ hour, block }: { hour: number; block: Block }) {
  const booked = "absolute inset-0 flex items-center rounded-md bg-primary px-3";
  return (
    <div
      className={cn(
        "relative h-12 w-[86%] rounded-md border text-sm font-medium",
        OPEN_STYLES[dayPhase(hour)],
      )}
    >
      <span className="flex h-full items-center px-3 text-foreground/60">open</span>
      {block.kind === "open" ? null : (
        <span
          style={
            block.kind === "booking"
              ? { animationDelay: `${block.delaySeconds}s` }
              : undefined
          }
          className={cn(
            booked,
            "origin-left text-primary-foreground",
            // Without motion a "booking" block simply stays open.
            block.kind === "booking" &&
              cn(
                "opacity-0 motion-safe:animate-tape-book",
                PAUSE_ON_HOVER,
                "group-data-[paused]:[animation-play-state:paused]",
              ),
          )}
        >
          {block.name}
        </span>
      )}
    </div>
  );
}

/**
 * The landing page's signature: a host's day spooling in, then sliding past
 * "now" with open slots getting booked. Purely illustrative, so it is hidden from
 * assistive tech.
 */
export function DayTape() {
  const ref = useRef<HTMLDivElement>(null);
  const [visible, setVisible] = useState(true);

  // Off screen, the loop has nobody to watch it.
  useEffect(() => {
    const element = ref.current;
    if (!element) return;
    const observer = new IntersectionObserver(([entry]) =>
      setVisible(entry?.isIntersecting ?? true),
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  return (
    <div
      ref={ref}
      aria-hidden="true"
      data-paused={visible ? undefined : ""}
      className="group relative overflow-hidden py-2 [mask-image:linear-gradient(to_right,transparent,black_7%,black_93%,transparent)]"
    >
      {/* The spool-in and the drift are two transforms, so each gets its own element. */}
      <div className="w-max motion-safe:animate-tape-spool">
        <div
          className={cn(
            "flex w-max motion-safe:animate-tape-drift",
            PAUSE_ON_HOVER,
            "group-data-[paused]:[animation-play-state:paused]",
          )}
        >
          {/* Two copies of the day make the loop seamless. */}
          {[0, 1].flatMap((copy) =>
            DAY.map(({ hour, block }) => (
              <div key={`${copy}-${hour}`} className="w-40 shrink-0 sm:w-48">
                {/* A sample day, rendered on the server: always the 24 hour clock. */}
                <HourMark hour={hour} hour12={false} size="lg" />
                <div className="mt-3 h-12">
                  {block ? <TapeBlock hour={hour} block={block} /> : null}
                </div>
              </div>
            )),
          )}
        </div>
      </div>

      {/* The tape moves; "now" stays put. */}
      <div className="pointer-events-none absolute inset-y-0 left-[30%] flex flex-col items-center">
        <span className="rounded-full bg-signal px-2 py-0.5 text-xs font-semibold text-signal-foreground">
          now
        </span>
        <span className="w-0.5 flex-1 bg-signal" />
      </div>
    </div>
  );
}
