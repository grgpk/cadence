import { createFileRoute, Link } from "@tanstack/react-router";
import { CalendarCheck } from "lucide-react";

import { Button } from "../components/ui/button";

export const Route = createFileRoute("/booking-confirmation")({
  component: BookingConfirmationPage,
});

function BookingConfirmationPage() {
  return (
    <main className="flex min-h-svh items-center justify-center bg-[#fff8f3] px-6 py-16">
      <section className="w-full max-w-xl rounded-2xl border bg-white p-8 text-center shadow-lg sm:p-12">
        <div className="mx-auto flex size-16 items-center justify-center rounded-full bg-green-100 text-green-700">
          <CalendarCheck className="size-8" />
        </div>
        <p className="mt-6 text-sm font-semibold tracking-wide text-muted-foreground uppercase">
          Cadence
        </p>
        <h1 className="mt-2 text-3xl font-black text-slate-900">Your call is booked.</h1>
        <p className="mt-4 leading-7 text-slate-600">
          Check your inbox for the confirmation and video-call details. We look forward to
          speaking with you.
        </p>
        <Button className="mt-8" asChild>
          <Link to="/">Back to Cadence</Link>
        </Button>
      </section>
    </main>
  );
}
