import { createServerFn } from "@tanstack/react-start";

import type { PlanOffer } from "../../bindings/PlanOffer";
import { SERVER_API_URL } from "../../lib/api-url";

/** Read on the server, so the prices are in the page's first HTML. */
export const getPlans = createServerFn({ method: "GET" }).handler(async () => {
  const response = await fetch(`${SERVER_API_URL}/api/billing/plans`);
  if (!response.ok) throw new Error("The plans did not load.");
  return response.json() as Promise<PlanOffer[]>;
});
