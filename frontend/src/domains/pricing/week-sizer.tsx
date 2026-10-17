import { useId } from "react";

import type { PlanOffer } from "../../bindings/PlanOffer";
import { Card } from "../../components/ui/card";
import { Label } from "../../components/ui/label";
import { Slider } from "../../components/ui/slider";
import { Switch } from "../../components/ui/switch";
import { fitReason, MAX_CALLS_PER_WEEK, type WeekNeeds } from "./pricing";

function callsLabel(calls: number): string {
  const count = calls >= MAX_CALLS_PER_WEEK ? `${MAX_CALLS_PER_WEEK}+` : String(calls);
  return `${count} ${calls === 1 ? "call" : "calls"} a week`;
}

/** The visitor describes their week; the page points to the plan that fits it. */
export function WeekSizer({
  offers,
  needs,
  onChange,
}: {
  offers: readonly PlanOffer[];
  needs: WeekNeeds;
  onChange: (needs: WeekNeeds) => void;
}) {
  const id = useId();
  const calls = needs.callsPerWeek;
  return (
    <Card className="gap-5 p-5 sm:p-6">
      <h2 id={`${id}-title`} className="text-lg font-semibold">
        How busy is your week?
      </h2>
      <div className="grid items-center gap-x-8 gap-y-4 md:grid-cols-[11rem_1fr]">
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
      <div className="flex items-center gap-3">
        <Switch
          id={`${id}-kinds`}
          checked={needs.severalKinds}
          onCheckedChange={(checked) => onChange({ ...needs, severalKinds: checked })}
        />
        <Label htmlFor={`${id}-kinds`} className="font-normal">
          I offer more than one kind of call
        </Label>
      </div>
      <p aria-live="polite" className="border-t pt-4 font-medium">
        {fitReason(offers, needs)}
      </p>
    </Card>
  );
}
