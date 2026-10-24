import { createServerFn } from "@tanstack/react-start";

import type { CryptoOptions } from "../../bindings/CryptoOptions";
import type { PlanOffer } from "../../bindings/PlanOffer";
import { SERVER_API_URL } from "../../lib/api-url";

/** Read on the server, so the prices are in the page's first HTML. */
export const getPlans = createServerFn({ method: "GET" }).handler(async () => {
  const response = await fetch(`${SERVER_API_URL}/api/billing/plans`);
  if (!response.ok) throw new Error("The plans did not load.");
  return response.json() as Promise<PlanOffer[]>;
});

const CRYPTO_OFF: CryptoOptions = { available: false, coins: null };

/** Whether paying with crypto is on. The page reads it as off when it cannot tell. */
export const getCryptoOptions = createServerFn({ method: "GET" }).handler(async () => {
  try {
    const response = await fetch(`${SERVER_API_URL}/api/billing/crypto/options`);
    return response.ok ? ((await response.json()) as CryptoOptions) : CRYPTO_OFF;
  } catch {
    return CRYPTO_OFF;
  }
});
