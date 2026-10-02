import { Link } from "@tanstack/react-router";
import { ArrowRight } from "lucide-react";

import { Button } from "../../components/ui/button";
import { CadenceBookingWidget } from "./CadenceBookingWidget";

export function PublicBookingPage({
  initialCountryAlpha2,
}: {
  initialCountryAlpha2?: string | null;
}) {
  return (
    <main className="min-h-svh bg-background text-foreground">
      <header className="sticky top-0 z-20 border-b bg-background/90 px-6 py-4 backdrop-blur">
        <div className="mx-auto flex max-w-6xl items-center justify-between">
          <Link to="/" className="text-lg font-bold text-foreground">
            Cadence
          </Link>
          <div className="flex items-center gap-3">
            <Button variant="outline" size="sm" asChild>
              <Link to="/login">Login</Link>
            </Button>
          </div>
        </div>
      </header>
      <section className="mx-auto max-w-6xl px-6 pb-16 pt-20">
        <div className="mx-auto max-w-4xl text-center">
          <p className="mx-auto inline-flex items-center gap-2 rounded-full border border-[#f52326]/20 bg-[#f52326]/5 px-3 py-1 text-xs font-semibold tracking-wide text-[#c51c1f] uppercase">
            <span className="size-1.5 rounded-full bg-[#f52326]" aria-hidden="true" />
            Simple scheduling for modern teams
          </p>
          <h1 className="mt-5 text-4xl font-black tracking-tight text-slate-950 sm:text-6xl">
            Book calls without the back-and-forth.
          </h1>
          <p className="mx-auto mt-5 max-w-2xl text-lg leading-8 text-slate-600">
            Cadence makes it easy to share your availability, collect the right context,
            and let people book a time that works for everyone.
          </p>
          <div className="mt-6 flex flex-wrap justify-center gap-x-6 gap-y-2 text-sm font-medium text-slate-500">
            <span>Share one link</span>
            <span>Collect useful details</span>
            <span>Keep every booking clear</span>
          </div>
          <Button
            className="mt-8 gap-2 bg-[#f52326] text-white hover:bg-[#d91f22]"
            size="lg"
            asChild
          >
            <a href="#book-your-call">
              Book a call <ArrowRight className="size-4" />
            </a>
          </Button>
        </div>
        <div
          id="book-your-call"
          className="mx-auto mt-12 w-full max-w-[1100px] scroll-mt-24"
        >
          <CadenceBookingWidget initialCountryAlpha2={initialCountryAlpha2} />
        </div>
      </section>
      <footer className="border-t px-6 py-8 text-sm text-slate-500">
        <div className="mx-auto flex max-w-6xl flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
          <span>© {new Date().getFullYear()} Cadence</span>
          <div className="flex gap-4">
            <Link to="/terms" className="hover:text-slate-900">
              Terms
            </Link>
            <Link to="/privacy-policy" className="hover:text-slate-900">
              Privacy
            </Link>
            <Link
              to="/login"
              className="inline-flex items-center gap-1 hover:text-slate-900"
            >
              Host workspace <ArrowRight className="size-3" />
            </Link>
          </div>
        </div>
      </footer>
    </main>
  );
}
