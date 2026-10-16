import { createFileRoute, Link } from "@tanstack/react-router";

import { ContactEmail, LegalPage, LegalSection } from "../domains/shared/legal-page";
import { APP_NAME } from "../lib/config";
import { ROUTES } from "../lib/routes";

export const Route = createFileRoute("/terms")({
  head: () => ({ meta: [{ title: `Terms of service | ${APP_NAME}` }] }),
  component: TermsPage,
});

function TermsPage() {
  return (
    <LegalPage title="Terms of service" updated="October 16, 2026">
      <LegalSection title="Introduction">
        <p>Welcome to {APP_NAME} ("Company", "we", "our", "us").</p>
        <p>
          These Terms of Service ("Terms") govern your use of the {APP_NAME} website,
          booking pages, and host dashboard (the "Service"), operated by {APP_NAME}.
        </p>
        <p>
          Our{" "}
          <Link
            to={ROUTES.privacy}
            className="text-foreground underline underline-offset-4"
          >
            Privacy Policy
          </Link>{" "}
          also governs your use of the Service and explains how we collect, protect, and
          share information that results from your use of it.
        </p>
        <p>
          By using the Service, you agree to these Terms and our Privacy Policy
          ("Agreements") and confirm that you have read and understood them.
        </p>
        <p>
          If you do not agree with the Agreements, please do not use the Service, and let
          us know at <ContactEmail /> so we can try to find a solution. These Terms apply
          to all visitors, hosts, invitees, and others who access or use the Service.
        </p>
      </LegalSection>

      <LegalSection title="Bookings">
        <p>
          Hosts publish booking pages where invitees choose a time and enter their
          details. A booking is confirmed when {APP_NAME} accepts the selected time.
          Invitees must provide accurate contact details. Hosts are responsible for
          attending, rescheduling, or cancelling the calls booked with them.
        </p>
      </LegalSection>

      <LegalSection title="Communications">
        <p>
          By booking a call or creating an account, you agree to receive service emails
          about your bookings and account, such as confirmations, reminders, changes, and
          password resets. We do not send marketing email without your consent. You can
          opt out of any non-essential email by writing to <ContactEmail />.
        </p>
      </LegalSection>

      <LegalSection title="Fair use">
        <p>
          Some {APP_NAME} features, such as email notifications, are sent from shared
          addresses and capacity used by many hosts at once. If a single user places
          disproportionately high demands on the Service, it may affect the experience of
          other users. {APP_NAME} may immediately suspend access to the Service if it
          reasonably believes that your use does or will adversely affect other users
          ("Fair Use"). Most users use the Service considerately, but even a few users
          acting inappropriately can significantly affect the Service for everyone else.
        </p>
      </LegalSection>

      <LegalSection title="Content">
        <p>
          The Service lets you publish and submit content such as booking page details,
          event descriptions, and answers to booking questions ("Content"). You are
          responsible for the Content you post or submit, including its legality,
          accuracy, reliability, and appropriateness.
        </p>
        <p>
          By posting Content, you represent and warrant that (i) you own it or have the
          rights, licenses, and permissions needed to use it and to let us use it as
          described in these Terms, and (ii) it does not violate the privacy, publicity,
          copyright, contract, or other rights of any person or entity. We may remove
          Content and terminate the account of any user found to infringe copyright.
        </p>
        <p>
          You keep all your rights to your Content and are responsible for protecting
          them. We take no responsibility and assume no liability for Content posted by
          you or any other user. By posting Content, you grant us a worldwide,
          non-exclusive, royalty-free license to use, reproduce, display, and distribute
          it on and through the Service only as needed to operate the Service, for example
          to show your booking page to invitees or your answers to the host.
        </p>
        <p>
          We have the right, but not the obligation, to monitor, edit, or remove Content
          posted on the Service. All other content on the Service is the property of{" "}
          {APP_NAME} or used with permission, and you may not copy, distribute, or reuse
          it for commercial purposes or personal gain without our written permission.
        </p>
      </LegalSection>

      <LegalSection title="Analytics">
        <p>
          We may use first-party data or third-party Service Providers to monitor and
          analyze use of the Service, as described in our Privacy Policy.
        </p>
      </LegalSection>

      <LegalSection title="No use by minors">
        <p>
          The Service is only for people who are at least 18 years old. By using the
          Service, you confirm that you are at least 18 and have the full authority,
          right, and capacity to enter into these Terms. If you are under 18, you may not
          access or use the Service.
        </p>
      </LegalSection>

      <LegalSection title="Accounts">
        <p>
          When you create an account, you must be over 18 and give us information that is
          accurate, complete, and current. Inaccurate, incomplete, or outdated information
          may lead to immediate termination of your account.
        </p>
        <p>
          You are responsible for keeping your account and password confidential and for
          restricting access to your account and devices. You accept responsibility for
          all activity under your account or password, whether or not you authorized it.
          Tell us immediately if you learn of any security breach or unauthorized use of
          your account.
        </p>
        <p>
          You may not use the name of another person or entity, or a name or trademark you
          are not authorized to use, as your account name or booking page. You may not use
          a name that is vulgar, offensive, or obscene.
        </p>
        <p>
          We may refuse service, terminate accounts, remove or edit content, or cancel
          bookings at our sole discretion.
        </p>
      </LegalSection>

      <LegalSection title="Intellectual property">
        <p>
          The Service and its original content (excluding Content provided by users),
          features, and functionality are and will remain the exclusive property of{" "}
          {APP_NAME} and its licensors. The Service is protected by copyright, trademark,
          and other laws. You may not use our trademarks and trade dress with any product
          or service without our prior written consent.
        </p>
      </LegalSection>

      <LegalSection title="Copyright policy">
        <p>
          We respect the intellectual property rights of others and respond to claims that
          Content posted on the Service infringes copyright or other intellectual property
          rights. If you believe your copyrighted work has been infringed, email{" "}
          <ContactEmail /> with the subject line "Copyright Infringement" and a detailed
          description of the alleged infringement.
        </p>
        <p>
          You may be held liable for damages, including legal fees, for bad-faith claims
          or misrepresentations of infringement.
        </p>
      </LegalSection>

      <LegalSection title="Error reporting and feedback">
        <p>
          You can send us feedback about errors, suggestions, ideas, problems, complaints,
          or other matters relating to the Service at <ContactEmail /> or through the
          bug-report tools in the Service ("Feedback"). By sending Feedback, you agree
          that (i) you do not keep any intellectual property or other right, title, or
          interest in it; (ii) we may already have similar development ideas; (iii) it
          contains no confidential or proprietary information from you or any third party;
          and (iv) we have no obligation of confidentiality for it. If mandatory law
          prevents the transfer of ownership of the Feedback to us, you grant us and our
          affiliates an exclusive, transferable, irrevocable, free-of-charge,
          sub-licensable, unlimited, and perpetual right to use it in any manner and for
          any purpose.
        </p>
      </LegalSection>

      <LegalSection title="Links to other websites">
        <p>
          The Service may link to third-party websites or services that we do not own or
          control, such as video-call links. We have no control over and take no
          responsibility for their content, privacy policies, or practices. A link does
          not mean we endorse the site, and we do not guarantee the accuracy or
          completeness of its information. You use such sites at your own risk.
        </p>
        <p>
          YOU ACKNOWLEDGE AND AGREE THAT {APP_NAME.toUpperCase()} IS NOT RESPONSIBLE OR
          LIABLE, DIRECTLY OR INDIRECTLY, FOR ANY DAMAGE OR LOSS CAUSED OR ALLEGED TO BE
          CAUSED BY OR IN CONNECTION WITH THE USE OF OR RELIANCE ON ANY CONTENT, GOODS, OR
          SERVICES AVAILABLE ON OR THROUGH ANY SUCH THIRD-PARTY WEBSITES OR SERVICES.
        </p>
        <p>
          WE STRONGLY ADVISE YOU TO READ THE TERMS OF SERVICE AND PRIVACY POLICIES OF ANY
          THIRD-PARTY WEBSITES OR SERVICES THAT YOU VISIT.
        </p>
      </LegalSection>

      <LegalSection title="Disclaimer of warranty">
        <p>
          THE SERVICE IS PROVIDED BY THE COMPANY ON AN "AS IS" AND "AS AVAILABLE" BASIS.
          THE COMPANY MAKES NO REPRESENTATIONS OR WARRANTIES OF ANY KIND, EXPRESS OR
          IMPLIED, AS TO THE OPERATION OF THE SERVICE OR THE INFORMATION, CONTENT, OR
          MATERIALS INCLUDED IN IT. YOU EXPRESSLY AGREE THAT YOUR USE OF THE SERVICE, ITS
          CONTENT, AND ANY SERVICES OR ITEMS OBTAINED FROM US IS AT YOUR SOLE RISK.
        </p>
        <p>
          NEITHER THE COMPANY NOR ANY PERSON ASSOCIATED WITH IT MAKES ANY WARRANTY OR
          REPRESENTATION ABOUT THE COMPLETENESS, SECURITY, RELIABILITY, QUALITY, ACCURACY,
          OR AVAILABILITY OF THE SERVICE. WITHOUT LIMITING THE FOREGOING, NEITHER THE
          COMPANY NOR ANYONE ASSOCIATED WITH IT REPRESENTS OR WARRANTS THAT THE SERVICE,
          ITS CONTENT, OR ANY SERVICES OR ITEMS OBTAINED THROUGH IT WILL BE ACCURATE,
          RELIABLE, ERROR-FREE, OR UNINTERRUPTED, THAT DEFECTS WILL BE CORRECTED, THAT THE
          SERVICE OR THE SERVER THAT MAKES IT AVAILABLE IS FREE OF VIRUSES OR OTHER
          HARMFUL COMPONENTS, OR THAT THE SERVICE WILL OTHERWISE MEET YOUR NEEDS OR
          EXPECTATIONS.
        </p>
        <p>
          THE COMPANY DISCLAIMS ALL WARRANTIES OF ANY KIND, WHETHER EXPRESS OR IMPLIED,
          STATUTORY, OR OTHERWISE, INCLUDING ANY WARRANTIES OF MERCHANTABILITY,
          NON-INFRINGEMENT, AND FITNESS FOR A PARTICULAR PURPOSE.
        </p>
        <p>
          THE FOREGOING DOES NOT AFFECT ANY WARRANTIES THAT CANNOT BE EXCLUDED OR LIMITED
          UNDER APPLICABLE LAW.
        </p>
      </LegalSection>

      <LegalSection title="Limitation of liability">
        <p>
          EXCEPT AS PROHIBITED BY LAW, YOU WILL HOLD US AND OUR OFFICERS, DIRECTORS,
          EMPLOYEES, AND AGENTS HARMLESS FOR ANY INDIRECT, PUNITIVE, SPECIAL, INCIDENTAL,
          OR CONSEQUENTIAL DAMAGE, HOWEVER IT ARISES (INCLUDING ATTORNEYS' FEES AND ALL
          RELATED COSTS AND EXPENSES OF LITIGATION AND ARBITRATION, OR AT TRIAL OR ON
          APPEAL, IF ANY, WHETHER OR NOT LITIGATION OR ARBITRATION IS INSTITUTED), WHETHER
          IN AN ACTION OF CONTRACT, NEGLIGENCE, OR OTHER TORTIOUS ACTION, OR ARISING OUT
          OF OR IN CONNECTION WITH THIS AGREEMENT, INCLUDING ANY CLAIM FOR PERSONAL INJURY
          OR PROPERTY DAMAGE, ARISING FROM THIS AGREEMENT AND ANY VIOLATION BY YOU OF ANY
          FEDERAL, STATE, OR LOCAL LAWS, STATUTES, RULES, OR REGULATIONS, EVEN IF THE
          COMPANY HAS BEEN PREVIOUSLY ADVISED OF THE POSSIBILITY OF SUCH DAMAGE. EXCEPT AS
          PROHIBITED BY LAW, IF THE COMPANY IS FOUND LIABLE, ITS LIABILITY WILL BE LIMITED
          TO THE AMOUNT YOU PAID FOR THE SERVICE, IF ANY, AND UNDER NO CIRCUMSTANCES WILL
          THERE BE CONSEQUENTIAL OR PUNITIVE DAMAGES. SOME STATES DO NOT ALLOW THE
          EXCLUSION OR LIMITATION OF PUNITIVE, INCIDENTAL, OR CONSEQUENTIAL DAMAGES, SO
          THE PRIOR LIMITATION OR EXCLUSION MAY NOT APPLY TO YOU.
        </p>
      </LegalSection>

      <LegalSection title="Termination">
        <p>
          We may terminate or suspend your account and access to the Service immediately,
          without notice or liability, at our sole discretion and for any reason,
          including a breach of these Terms.
        </p>
        <p>
          To end your account, you can stop using the Service or ask us to delete it at{" "}
          <ContactEmail />.
        </p>
        <p>
          Provisions of these Terms that by their nature should survive termination will
          survive, including ownership provisions, warranty disclaimers, indemnity, and
          limitations of liability.
        </p>
      </LegalSection>

      <LegalSection title="Changes to the service and these terms">
        <p>
          We may change or withdraw the Service, or any part of it, at our discretion
          without notice. We are not liable if all or part of the Service is unavailable
          at any time or for any period. From time to time, we may restrict access to some
          or all of the Service to certain users, including registered users.
        </p>
        <p>
          We may change these Terms at any time by posting the revised version on this
          page. Please review these Terms periodically.
        </p>
        <p>
          By continuing to use the Service after revised Terms take effect, you agree to
          them. If you do not agree to the new Terms, you are no longer authorized to use
          the Service.
        </p>
      </LegalSection>

      <LegalSection title="Waiver and severability">
        <p>
          No waiver by the Company of any term or condition in these Terms is a further or
          continuing waiver of that term or of any other term, and any failure of the
          Company to assert a right or provision under these Terms does not waive that
          right or provision.
        </p>
        <p>
          If a court or other tribunal of competent jurisdiction holds any provision of
          these Terms invalid, illegal, or unenforceable, that provision will be removed
          or limited to the minimum extent needed, and the remaining provisions will stay
          in full force and effect.
        </p>
      </LegalSection>

      <LegalSection title="Acknowledgment">
        <p>
          BY USING THE SERVICE, YOU ACKNOWLEDGE THAT YOU HAVE READ THESE TERMS OF SERVICE
          AND AGREE TO BE BOUND BY THEM.
        </p>
      </LegalSection>

      <LegalSection title="Contact us">
        <p>
          If you have questions about these Terms, email us at <ContactEmail />.
        </p>
      </LegalSection>
    </LegalPage>
  );
}
