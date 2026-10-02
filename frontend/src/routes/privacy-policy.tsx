import { createFileRoute, Link } from "@tanstack/react-router";

export const Route = createFileRoute("/privacy-policy")({ component: PrivacyPolicyPage });

function PrivacyPolicyPage() {
  return (
    <main className="min-h-svh bg-[#fff8f3] px-6 py-16 text-slate-900">
      <article className="mx-auto max-w-3xl rounded-2xl border bg-white p-8 shadow-sm sm:p-12">
        <Link to="/" className="text-sm font-bold text-foreground">
          Cadence
        </Link>
        <h1 className="mt-8 text-4xl font-black">Privacy Policy</h1>
        <p className="mt-3 text-sm text-slate-500">Last updated: October 1, 2026</p>
        <div className="mt-10 space-y-8 leading-7 text-slate-700">
          <section>
            <h2 className="text-2xl font-bold text-slate-900">Data we collect</h2>
            <p className="mt-3">
              We collect information you submit through the booking form, including name,
              email, phone, country, answers, selected timezone, and booking details. We
              also collect technical and attribution data such as source page, browser
              information, and campaign parameters.
            </p>
          </section>
          <section>
            <h2 className="text-2xl font-bold text-slate-900">How we use data</h2>
            <p className="mt-3">
              We use data to qualify requests, provide coaching calls, create calendar
              events, send confirmations and reminders, prevent abuse, measure funnel
              performance, and improve Cadence.
            </p>
          </section>
          <section>
            <h2 className="text-2xl font-bold text-slate-900">Service providers</h2>
            <p className="mt-3">
              We may share necessary data with calendar, video-call, email, hosting,
              database, analytics, and error-monitoring providers. Providers receive only
              data needed for their service.
            </p>
          </section>
          <section>
            <h2 className="text-2xl font-bold text-slate-900">Retention and rights</h2>
            <p className="mt-3">
              We retain data while needed for the purposes above, legal obligations,
              disputes, and security. Depending on your location, you may request access,
              correction, deletion, restriction, portability, or withdrawal of consent.
            </p>
          </section>
          <section>
            <h2 className="text-2xl font-bold text-slate-900">Cookies and analytics</h2>
            <p className="mt-3">
              Cadence may use cookies, local storage, and analytics events for session
              security, preferences, attribution, and service improvement.
            </p>
          </section>
          <section>
            <h2 className="text-2xl font-bold text-slate-900">Contact</h2>
            <p className="mt-3">
              For privacy requests, contact the Cadence operator through the email address
              shown in your booking communication.
            </p>
          </section>
        </div>
        <div className="mt-12 flex gap-4 text-sm">
          <Link to="/terms" className="text-foreground hover:underline">
            Terms of service
          </Link>
          <Link to="/book-a-call" className="text-foreground hover:underline">
            Book a call
          </Link>
        </div>
      </article>
    </main>
  );
}
