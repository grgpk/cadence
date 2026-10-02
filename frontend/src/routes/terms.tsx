import { createFileRoute, Link } from "@tanstack/react-router";

export const Route = createFileRoute("/terms")({ component: TermsPage });

function TermsPage() {
  return (
    <LegalPage title="Terms of Service" updated="October 1, 2026">
      <h2>Using Cadence</h2>
      <p>
        Cadence provides scheduling tools and coaching-call services. By using Cadence,
        you agree to use the service lawfully and provide accurate information.
      </p>
      <h2>Bookings</h2>
      <p>
        A booking is requested through the public widget and confirmed after Cadence
        accepts the selected slot. You must provide a valid name, email, and contact
        details.
      </p>
      <h2>Communications</h2>
      <p>
        We may send booking confirmations, reminders, and service-related messages. You
        can request marketing opt-out at any time.
      </p>
      <h2>Acceptable use</h2>
      <p>
        Do not abuse the service, attempt unauthorized access, submit misleading data, or
        interfere with calendar, email, or booking systems.
      </p>
      <h2>Third-party services</h2>
      <p>
        Calendar, video-call, email, analytics, and hosting providers may process data
        needed to operate Cadence.
      </p>
      <h2>Contact</h2>
      <p>
        Questions about these terms: contact the Cadence operator through the email
        address shown in your booking communication.
      </p>
    </LegalPage>
  );
}

function LegalPage({
  title,
  updated,
  children,
}: {
  title: string;
  updated: string;
  children: React.ReactNode;
}) {
  return (
    <main className="min-h-svh bg-[#fff8f3] px-6 py-16 text-slate-900">
      <article className="mx-auto max-w-3xl rounded-2xl border bg-white p-8 shadow-sm sm:p-12">
        <Link to="/" className="text-sm font-bold text-foreground">
          Cadence
        </Link>
        <h1 className="mt-8 text-4xl font-black">{title}</h1>
        <p className="mt-3 text-sm text-slate-500">Last updated: {updated}</p>
        <div className="prose prose-slate mt-10 max-w-none space-y-8 leading-7">
          {children}
        </div>
        <div className="mt-12 flex gap-4 text-sm">
          <Link to="/privacy-policy" className="text-foreground hover:underline">
            Privacy policy
          </Link>
          <Link to="/book-a-call" className="text-foreground hover:underline">
            Book a call
          </Link>
        </div>
      </article>
    </main>
  );
}
