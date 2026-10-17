import { useQuery } from "@tanstack/react-query";
import { Link, useRouterState } from "@tanstack/react-router";
import { CalendarCheck, CircleAlert, Clock } from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "../../components/ui/alert";
import { Button } from "../../components/ui/button";
import { ROUTES } from "../../lib/routes";
import { billingQuery } from "./billing-api";
import { bookingUsage, STATUS, TRIAL_WARNING_DAYS, trialDaysLeft } from "./billing";

/** The banner names the first few; Billing lists them all. */
const TRIAL_LOSSES_SHOWN = 3;

/**
 * Above every dashboard page: a payment that failed, a month past the
 * bookings the plan includes, or a trial about to end.
 */
export function PlanBanner() {
  const { data: summary } = useQuery(billingQuery);
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  if (!summary) return null;

  if (summary.status === STATUS.PAST_DUE) {
    return (
      <Alert variant="destructive">
        <CircleAlert className="size-4" />
        <AlertTitle>Your last payment failed</AlertTitle>
        <AlertDescription>
          <p>Update your card to keep your plan.</p>
          <Button variant="outline" size="sm" className="mt-2" asChild>
            <Link to={ROUTES.dashboardBilling}>Go to billing</Link>
          </Button>
        </AlertDescription>
      </Alert>
    );
  }

  if (bookingUsage(summary) === 1) {
    return (
      <Alert>
        <CalendarCheck className="size-4" />
        <AlertTitle>
          You've had {summary.bookings_this_month} bookings this month
        </AlertTitle>
        <AlertDescription>
          <p>
            Free includes {summary.entitlements.included_bookings_per_month} a month. Your
            page stays open, so nobody is turned away. Upgrade for more reminders, your
            own questions and no Cadence logo.
          </p>
          <Button size="sm" className="mt-2" asChild>
            <Link to={ROUTES.dashboardBilling}>See plans</Link>
          </Button>
        </AlertDescription>
      </Alert>
    );
  }

  const days = trialDaysLeft(summary, Date.now());
  if (days === null || days > TRIAL_WARNING_DAYS) return null;
  // Billing says the same in its own panel, with the plans right below it.
  if (pathname === ROUTES.dashboardBilling) return null;
  return (
    <Alert>
      <Clock className="size-4" />
      <AlertTitle>
        {days} {days === 1 ? "day" : "days"} left of your Pro trial
      </AlertTitle>
      <AlertDescription>
        {summary.trial_losses.length > 0 ? (
          <>
            <p>After it ends, on Free:</p>
            <ul className="mt-1 list-disc pl-5">
              {summary.trial_losses.slice(0, TRIAL_LOSSES_SHOWN).map((loss) => (
                <li key={loss}>{loss}</li>
              ))}
            </ul>
            <p className="mt-1">Your settings stay saved.</p>
          </>
        ) : (
          <p>After it ends you move to Free. Your settings stay saved.</p>
        )}
        <Button size="sm" className="mt-2" asChild>
          <Link to={ROUTES.dashboardBilling}>Pick a plan</Link>
        </Button>
      </AlertDescription>
    </Alert>
  );
}
