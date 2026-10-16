import { createFileRoute } from "@tanstack/react-router";

import {
  ContactEmail,
  LegalList,
  LegalPage,
  LegalSection,
  LegalSubsection,
} from "../domains/shared/legal-page";
import { APP_NAME } from "../lib/config";

export const Route = createFileRoute("/privacy-policy")({
  head: () => ({ meta: [{ title: `Privacy policy | ${APP_NAME}` }] }),
  component: PrivacyPolicyPage,
});

function PrivacyPolicyPage() {
  return (
    <LegalPage title="Privacy policy" updated="October 16, 2026">
      <LegalSection title="Introduction">
        <p>Welcome to {APP_NAME}.</p>
        <p>
          {APP_NAME} ("us", "we", or "our") operates the {APP_NAME} website, booking
          pages, and host dashboard, referred to as the "Service" from here on.
        </p>
        <p>
          This Privacy Policy governs your visit to the Service and explains how we
          collect, protect, and disclose information that results from your use of the
          Service.
        </p>
        <p>
          We use your data to provide and improve the Service. By using the Service, you
          agree to the collection and use of information in accordance with this policy.
          Unless defined otherwise here, terms in this policy have the same meanings as in
          our Terms of Service.
        </p>
        <p>
          Our Terms of Service ("Terms") govern all use of the Service. Together with this
          Privacy Policy, they form your agreement with us.
        </p>
      </LegalSection>

      <LegalSection title="Definitions">
        <p>
          <strong>Service</strong> means the {APP_NAME} website, booking pages, and host
          dashboard operated by {APP_NAME}.
        </p>
        <p>
          <strong>Personal Data</strong> means any information about an identified or
          identifiable living individual, including data we already hold or may obtain in
          the future.
        </p>
        <p>
          <strong>Cookies</strong> are small files stored on your computer or mobile
          device.
        </p>
        <p>
          <strong>Data Controller</strong> means the person who determines the purposes
          and means of processing Personal Data. For the purposes of this policy, we are
          the Data Controller of your data.
        </p>
        <p>
          <strong>Data Processors (Service Providers)</strong> means any person who
          processes data on behalf of the Data Controller. We may use various Service
          Providers to process your data more effectively.
        </p>
        <p>
          <strong>Data Subject</strong> means any living individual who is the subject of
          Personal Data.
        </p>
        <p>
          <strong>User</strong> means the individual using the Service, whether as a host
          or as an invitee booking a call. The User is the Data Subject.
        </p>
      </LegalSection>

      <LegalSection title="Information collection and use">
        <p>We collect several types of information for different purposes.</p>
        <LegalSubsection title="Personal Data">
          <p>
            When you use the Service, we may ask for personally identifiable information
            that can be used to contact or identify you. When you book a call, this
            includes your name, email address, phone number, country, timezone, answers to
            booking questions, and the time you choose. When you create a host account, it
            includes your name, email address, and password, which is stored hashed.
          </p>
        </LegalSubsection>
        <LegalSubsection title="How we collect personal information">
          <p>We collect information you provide when you:</p>
          <LegalList
            items={[
              "Visit our website or use the Service",
              "Register for a host account",
              "Enter your details in a booking form",
              "Contact us or send a bug report",
            ]}
          />
          <p>We may also receive information from:</p>
          <LegalList
            items={[
              "Calendar and video-call providers you connect to your account",
              "Security, abuse prevention, and infrastructure providers",
            ]}
          />
        </LegalSubsection>
        <LegalSubsection title="Legitimate interest in capturing contact information">
          <p>
            In some cases, we store information such as your name, email address, or phone
            number when you enter it into a booking form field and move away from that
            field without completing the booking. Data is saved only after you finish with
            the field, not while you type. We process this limited information under our
            legitimate interest in scheduling continuity, reducing data loss, and
            improving the booking experience. It is stored securely and used only to
            continue or complete the intended booking. You can ask us to delete it at any
            time by contacting us at <ContactEmail />.
          </p>
        </LegalSubsection>
      </LegalSection>

      <LegalSection title="Usage data">
        <p>
          We may also collect Usage Data that your browser sends or that our
          infrastructure records when you use the Service. This may include:
        </p>
        <LegalList
          items={[
            "Your IP address, browser type, and browser version",
            "Pages of the Service you visit and the page you came from",
            "Campaign and attribution parameters in the page URL",
            "The time and date of your visit and time spent on pages",
            "Device identifiers and other diagnostic data",
          ]}
        />
      </LegalSection>

      <LegalSection title="Cookies and local storage">
        <p>
          We use cookies and browser storage to keep the Service working and to remember
          your choices. You can tell your browser to refuse cookies or to warn you when
          one is sent, but parts of the Service, such as signing in, may not work without
          them.
        </p>
        <p>Examples of cookies and storage we use:</p>
        <LegalList
          items={[
            "Session cookies: keep you signed in to your host account",
            "Preference storage: remembers settings such as theme and timezone",
            "Security cookies: protect accounts and prevent abuse",
          ]}
        />
      </LegalSection>

      <LegalSection title="Use of data">
        <p>{APP_NAME} uses the data it collects to:</p>
        <LegalList
          items={[
            "Provide and maintain the Service",
            "Schedule booked calls, create calendar events and video links, and send confirmations, reminders, and changes",
            "Share your booking details with the host you booked with",
            "Notify you about changes to the Service",
            "Provide support",
            "Analyze and improve the Service",
            "Monitor use of the Service",
            "Detect, prevent, and address technical issues and abuse",
            "Send notices about your account, including security and password emails",
            "Fulfill any other purpose for which you provide the data",
            "Meet our obligations and enforce our rights under any agreement between you and us",
            "Use the data for any other purpose with your consent",
          ]}
        />
      </LegalSection>

      <LegalSection title="Retention and transfer of data">
        <p>
          We keep Personal Data only as long as needed for the purposes in this policy,
          and to the extent needed to meet our legal obligations, resolve disputes, and
          enforce our agreements and policies.
        </p>
        <p>
          We may keep Usage Data for internal analysis. It is usually kept for a shorter
          time unless we need it to improve the security or functionality of the Service,
          or the law requires us to keep it longer.
        </p>
        <p>
          Your information, including Personal Data, may be transferred to and stored on
          computers outside your state, province, country, or other jurisdiction, where
          data protection laws may differ from yours. By agreeing to this policy and
          submitting your information, you agree to that transfer.
        </p>
        <p>
          {APP_NAME} will take all reasonable steps to treat your data securely and in
          line with this policy. We will not transfer your Personal Data to an
          organization or country unless adequate controls are in place, including for the
          security of your data.
        </p>
      </LegalSection>

      <LegalSection title="Disclosure of data">
        <p>We may disclose personal information we collect or you provide:</p>
        <LegalSubsection title="Disclosure for law enforcement">
          <p>
            We may be required to disclose your Personal Data by law or in response to
            valid requests from public authorities.
          </p>
        </LegalSubsection>
        <LegalSubsection title="Business transaction">
          <p>
            If we are involved in a merger, acquisition, or asset sale, your Personal Data
            may be transferred.
          </p>
        </LegalSubsection>
        <LegalSubsection title="Other cases">
          <p>We may also disclose your information:</p>
          <LegalList
            items={[
              "To the host whose booking page you used, so they can hold the call",
              "To contractors, service providers, and other third parties who support the Service",
              "To fulfill the purpose for which you provided it",
              "For any other purpose we disclose when you provide it",
              "With your consent in any other case",
              `If we believe disclosure is needed to protect the rights, property, or safety of ${APP_NAME}, our users, or others`,
            ]}
          />
        </LegalSubsection>
      </LegalSection>

      <LegalSection title="Data security">
        <p>
          The security of your data matters to us, but no method of transmission over the
          internet or electronic storage is completely secure. We use commercially
          acceptable means to protect your Personal Data but cannot guarantee its absolute
          security.
        </p>
      </LegalSection>

      <LegalSection title="Data protection rights">
        <p>
          If you live in the European Union (EU) or European Economic Area (EEA), you have
          rights under the General Data Protection Regulation (GDPR). We take reasonable
          steps to let you correct, amend, delete, or limit the use of your Personal Data.
          To learn what Personal Data we hold about you or to have it removed, email us at{" "}
          <ContactEmail />.
        </p>
        <p>In certain circumstances, you have the right to:</p>
        <LegalList
          items={[
            "Access, update, or delete the information we hold about you",
            "Rectification: have inaccurate or incomplete information corrected",
            "Object to our processing of your Personal Data",
            "Restriction: ask us to restrict processing of your Personal Data",
            "Data portability: receive a copy of your data in a structured, machine-readable, commonly used format",
            "Withdraw consent at any time where we rely on your consent",
          ]}
        />
        <p>
          We may need to verify your identity before responding. Some data may be required
          for the Service to work.
        </p>
        <LegalSubsection title="California residents">
          <p>
            If you live in California, the California Consumer Privacy Act (CCPA) gives
            you the right to know what data we collect about you, to ask us to delete it,
            and to stop us from selling or sharing it. You can request:
          </p>
          <LegalList
            items={[
              "The personal information we hold about you",
              "Deletion of your personal information",
              "That we stop selling or sharing your personal information",
            ]}
          />
          <p>
            Send requests to <ContactEmail />. We will not discriminate against you for
            exercising these rights. We do not sell personal information.
          </p>
        </LegalSubsection>
      </LegalSection>

      <LegalSection title="Service providers">
        <p>
          We use third-party companies to run the Service, perform tasks for us, or help
          us understand how the Service is used. These include calendar and video-call,
          email delivery, hosting, and database providers. They can access your Personal
          Data only to do these tasks and must not disclose or use it for anything else.
        </p>
        <p>
          When a host connects Google Calendar, {APP_NAME} uses Google API Services. Our
          use of information received from Google APIs follows the{" "}
          <a
            href="https://developers.google.com/terms/api-services-user-data-policy"
            className="text-foreground underline underline-offset-4"
            target="_blank"
            rel="noreferrer"
          >
            Google API Services User Data Policy
          </a>
          , including the Limited Use requirements.
        </p>
      </LegalSection>

      <LegalSection title="Analytics">
        <p>
          We may use first-party data or third-party Service Providers to monitor and
          analyze use of the Service.
        </p>
      </LegalSection>

      <LegalSection title="Links to other sites">
        <p>
          The Service may contain links to sites we do not operate, such as video-call
          links. If you follow one, you will go to that third party's site. We strongly
          recommend reading the privacy policy of every site you visit.
        </p>
        <p>
          We have no control over, and take no responsibility for, the content, privacy
          policies, or practices of third-party sites or services.
        </p>
      </LegalSection>

      <LegalSection title="Children's privacy">
        <p>The Service is not intended for anyone under 18 ("Child" or "Children").</p>
        <p>
          We do not knowingly collect personally identifiable information from Children.
          If you know a Child has given us Personal Data, please contact us. If we learn
          we have collected Personal Data from Children without verified parental consent,
          we take steps to remove it from our servers.
        </p>
      </LegalSection>

      <LegalSection title="Changes to this privacy policy">
        <p>
          We may update this Privacy Policy from time to time. We will post the new
          version on this page and update the "Last updated" date at the top.
        </p>
        <p>
          We will let you know by email and/or a prominent notice on the Service before
          material changes take effect. Please review this policy periodically. Changes
          take effect when they are posted on this page.
        </p>
      </LegalSection>

      <LegalSection title="Contact us">
        <p>
          If you have questions about this Privacy Policy, email us at <ContactEmail />.
        </p>
      </LegalSection>
    </LegalPage>
  );
}
