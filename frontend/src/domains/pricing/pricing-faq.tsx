import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "../../components/ui/accordion";

const QUESTIONS = [
  {
    question: "How does the free trial work?",
    answer:
      "Every new account gets Pro for 14 days, no card needed. When the trial ends you move to Free unless you pick a plan. Nothing you set up is deleted.",
  },
  {
    question: "What counts as a booking?",
    answer:
      "Each call someone books on your page in a calendar month, in your timezone. Cancelled calls do not count.",
  },
  {
    question: "Can I change or cancel my plan later?",
    answer:
      "Yes, at any time from your dashboard. A change between plans is prorated. When you cancel, your plan stays until the end of the period you paid for.",
  },
  {
    question: "What happens after 25 bookings on Free?",
    answer:
      "Your page stays open and people keep booking. Nobody is turned away. We email you once that month so you can decide whether Pro is worth it.",
  },
  {
    question: "Will my price go up?",
    answer:
      "Not while you stay subscribed. If our prices change, they change for new subscriptions only. You keep the price you signed up at.",
  },
  {
    question: "What happens when I go back to Free?",
    answer:
      "Nothing is deleted. Your settings, questions and calls stay saved. Paid features, such as extra reminders and your own form questions, pause until you upgrade again.",
  },
  {
    question: "Can I bring my setup from another scheduler?",
    answer:
      "Yes. From Calendly, paste a personal access token and your event types, weekly hours, days off and timezone come over in a minute. Your Calendly account is not changed.",
  },
  {
    question: "Do you give refunds?",
    answer:
      "We do not refund part of a period. When you cancel, you keep your plan until the period you paid for ends.",
  },
  {
    question: "Are taxes included?",
    answer:
      "Prices are in US dollars. Depending on where you are, tax may be added at checkout.",
  },
];

export function PricingFaq() {
  return (
    <Accordion type="single" collapsible>
      {QUESTIONS.map(({ question, answer }) => (
        <AccordionItem key={question} value={question}>
          <AccordionTrigger className="text-base">{question}</AccordionTrigger>
          <AccordionContent className="max-w-2xl text-muted-foreground">
            {answer}
          </AccordionContent>
        </AccordionItem>
      ))}
    </Accordion>
  );
}
