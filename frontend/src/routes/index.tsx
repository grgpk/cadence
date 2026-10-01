import { createFileRoute, Link } from "@tanstack/react-router";
import { ArrowRight } from "lucide-react";

import { Button } from "../components/ui/button";
import { BookingWidget } from "../domains/widget/BookingWidget";
import { PUBLIC_HOST_UNID } from "../lib/config";

export const Route = createFileRoute("/")({ component: HomePage });

function HomePage() {
  return (
    <main className="min-h-svh bg-background">
      <header className="mx-auto flex max-w-7xl items-center justify-between px-6 py-5">
        <strong className="text-lg">Cadence</strong>
        <Button variant="outline" size="sm" asChild>
          <Link to="/login">Host login</Link>
        </Button>
      </header>
      <section className="mx-auto grid max-w-7xl items-center gap-12 px-6 py-16 lg:grid-cols-[1fr_minmax(320px,420px)]">
        <div className="space-y-5">
          <p className="text-sm font-medium text-muted-foreground">
            Simple booking cadence.
          </p>
          <h1 className="max-w-xl text-4xl font-semibold tracking-tight sm:text-5xl">
            Book the right call at the right time.
          </h1>
          <p className="max-w-lg text-lg text-muted-foreground">
            Cadence gives hosts a clean booking widget and a focused dashboard.
          </p>
          <Button variant="ghost" className="px-0" asChild>
            <Link to="/login">
              Open host workspace <ArrowRight className="size-4" />
            </Link>
          </Button>
        </div>
        <BookingWidget hostUnid={PUBLIC_HOST_UNID} />
      </section>
    </main>
  );
}
