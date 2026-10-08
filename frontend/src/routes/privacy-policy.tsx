import { createFileRoute } from "@tanstack/react-router";

import { LegalPage, LegalSection } from "../domains/shared/LegalPage";
import { APP_NAME } from "../lib/config";

export const Route = createFileRoute("/privacy-policy")({
  head: () => ({ meta: [{ title: `Privacy policy | ${APP_NAME}` }] }),
  component: PrivacyPolicyPage,
});

function PrivacyPolicyPage() {
  return (
    <LegalPage title="Privacy policy" updated="October 9, 2026">
      <LegalSection title="Data we collect">
        We collect information you submit through the booking form, including name, email,
        phone, country, answers, selected timezone, and booking details. We also collect
        technical and attribution data such as source page, browser information, and
        campaign parameters.
      </LegalSection>
      <LegalSection title="How we use data">
        We use data to arrange the calls you book, create calendar events, send
        confirmations and reminders, prevent abuse, measure how the service is used, and
        improve {APP_NAME}.
      </LegalSection>
      <LegalSection title="Service providers">
        We may share necessary data with calendar, video-call, email, hosting, database,
        analytics, and error-monitoring providers. Providers receive only data needed for
        their service.
      </LegalSection>
      <LegalSection title="Retention and rights">
        We retain data while needed for the purposes above, legal obligations, disputes,
        and security. Depending on your location, you may request access, correction,
        deletion, restriction, portability, or withdrawal of consent.
      </LegalSection>
      <LegalSection title="Cookies and analytics">
        {APP_NAME} may use cookies, local storage, and analytics events for session
        security, preferences, attribution, and service improvement.
      </LegalSection>
      <LegalSection title="Contact">
        For privacy requests, contact the {APP_NAME} operator through the email address
        shown in your booking communication.
      </LegalSection>
    </LegalPage>
  );
}
