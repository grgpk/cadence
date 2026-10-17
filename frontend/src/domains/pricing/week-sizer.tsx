import { type ReactNode, useId } from "react";

import type { PlanOffer } from "../../bindings/PlanOffer";
import { Card } from "../../components/ui/card";
import { Label } from "../../components/ui/label";
import { Slider } from "../../components/ui/slider";
import { Switch } from "../../components/ui/switch";
import { fitReason, MAX_CALLS_PER_WEEK, type WeekNeeds } from "./pricing";
import { WeekGrid } from "./week-grid";

function callsLabel(calls: number): string {
  const count = calls >= MAX_CALLS_PER_WEEK ? `${MAX_CALLS_PER_WEEK}+` : String(calls);
  return `${count} ${calls === 1 ? "call" : "calls"} a week`;
}

/** The visitor describes their week; the page points to the plan that fits it. */
export function WeekSizer({
  offers,
  needs,
  onChange,
  action,
}: {
  offers: readonly PlanOffer[];
  needs: WeekNeeds;
  onChange: (needs: WeekNeeds) => void;
  /** Starts the plan that fits, next to the reason it fits. */
  action: ReactNode;
}) {
  const id = useId();
  const calls = needs.callsPerWeek;
  return (
    <Card className="gap-5 p-5 sm:p-6">
      <h2 id={`${id}-title`} className="text-lg font-semibold">
        How busy is your week?
      </h2>
      <div className="grid items-center gap-x-8 gap-y-4 sm:grid-cols-[11rem_1fr]">
        <p className="flex items-baseline gap-2" aria-hidden>
          <span className="text-clock text-5xl font-semibold tracking-tight">
            {calls >= MAX_CALLS_PER_WEEK ? `${MAX_CALLS_PER_WEEK}+` : calls}
          </span>
          <span className="text-muted-foreground">
            {calls === 1 ? "call a week" : "calls a week"}
          </span>
        </p>
        <Slider
          min={1}
          max={MAX_CALLS_PER_WEEK}
          step={1}
          value={[calls]}
          onValueChange={([value]) =>
            onChange({ ...needs, callsPerWeek: value ?? calls })
          }
          aria-labelledby={`${id}-title`}
          thumbProps={{
            "aria-label": "Calls a week",
            "aria-valuetext": callsLabel(calls),
          }}
          className="py-2"
        />
      </div>
      <WeekGrid calls={calls} />
      <div className="grid gap-4 sm:grid-cols-2 sm:gap-6">
        <div className="flex items-start gap-3">
          <Switch
            id={`${id}-kinds`}
            checked={needs.severalKinds}
            onCheckedChange={(checked) => onChange({ ...needs, severalKinds: checked })}
          />
          <Label htmlFor={`${id}-kinds`} className="font-normal leading-snug">
            I offer more than one kind of call
          </Label>
        </div>
        <div className="flex items-start gap-3">
          <Switch
            id={`${id}-tools`}
            checked={needs.clientTools}
            onCheckedChange={(checked) => onChange({ ...needs, clientTools: checked })}
          />
          <Label htmlFor={`${id}-tools`} className="font-normal leading-snug">
            I want to follow up with people and see who shows up
          </Label>
        </div>
      </div>
      <div className="flex flex-wrap items-center justify-between gap-x-6 gap-y-3 border-t pt-4">
        <p aria-live="polite" className="max-w-md font-medium">
          {fitReason(offers, needs)}
        </p>
        {action}
      </div>
    </Card>
  );
}
