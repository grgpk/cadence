import { createFileRoute } from "@tanstack/react-router";

import { LegalPage, LegalSection } from "../domains/shared/LegalPage";
import { APP_NAME } from "../lib/config";

export const Route = createFileRoute("/terms")({
  head: () => ({ meta: [{ title: `Terms of service | ${APP_NAME}` }] }),
  component: TermsPage,
});

function TermsPage() {
  return (
    <LegalPage title="Terms of service" updated="October 1, 2026">
      <LegalSection title="Using Cadence">
        Cadence provides scheduling tools and coaching-call services. By using Cadence,
        you agree to use the service lawfully and provide accurate information.
      </LegalSection>
      <LegalSection title="Bookings">
        A booking is requested through the public widget and confirmed after Cadence
        accepts the selected slot. You must provide a valid name, email, and contact
        details.
      </LegalSection>
      <LegalSection title="Communications">
        We may send booking confirmations, reminders, and service-related messages. You
        can request marketing opt-out at any time.
      </LegalSection>
      <LegalSection title="Acceptable use">
        Do not abuse the service, attempt unauthorized access, submit misleading data, or
        interfere with calendar, email, or booking systems.
      </LegalSection>
      <LegalSection title="Third-party services">
        Calendar, video-call, email, analytics, and hosting providers may process data
        needed to operate Cadence.
      </LegalSection>
      <LegalSection title="Contact">
        Questions about these terms: contact the Cadence operator through the email
        address shown in your booking communication.
      </LegalSection>
    </LegalPage>
  );
}
