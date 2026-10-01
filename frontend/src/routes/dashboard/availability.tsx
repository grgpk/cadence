import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import type { AvailabilityRule } from "../../bindings/AvailabilityRule";
import { api } from "../../lib/api";

export const Route = createFileRoute("/dashboard/availability")({ component: AvailabilityPage });
function AvailabilityPage() { const [rules, setRules] = useState<AvailabilityRule[]>([]); useEffect(() => { api<AvailabilityRule[]>("/api/availability").then(setRules).catch(() => setRules([])); }, []); return <section><h1>Availability</h1><div className="card">{rules.length === 0 ? <p>No availability rules yet.</p> : rules.map((rule) => <p key={rule.id}>Day {rule.weekday}, {rule.start_time} to {rule.end_time}, {rule.slot_minutes} min</p>)}</div></section>; }
