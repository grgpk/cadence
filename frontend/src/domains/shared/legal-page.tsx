import { type ReactNode, type RefObject, useEffect, useRef, useState } from "react";

import { CONTACT_EMAIL } from "../../lib/config";
import { PublicLayout } from "./public-layout";

/** A section's anchor: "Your rights" becomes "your-rights". */
function sectionId(title: string): string {
  return title
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
}

/** The sections on the page, read once it is mounted, for the contents list. */
function useSectionTitles(article: RefObject<HTMLElement | null>) {
  const [titles, setTitles] = useState<{ id: string; title: string }[]>([]);
  useEffect(() => {
    const headings = article.current?.querySelectorAll("h2[id]") ?? [];
    setTitles(
      Array.from(headings, (heading) => ({
        id: heading.id,
        title: heading.textContent ?? "",
      })),
    );
  }, [article]);
  return titles;
}

export function LegalPage({
  title,
  updated,
  children,
}: {
  title: string;
  updated: string;
  children: ReactNode;
}) {
  const article = useRef<HTMLElement>(null);
  const titles = useSectionTitles(article);
  return (
    <PublicLayout>
      <div className="mx-auto grid max-w-[82rem] gap-12 px-4 py-12 sm:px-6 sm:py-20 lg:grid-cols-[14rem_minmax(0,1fr)]">
        <nav aria-label="On this page" className="max-lg:hidden">
          {titles.length > 0 ? (
            <ul className="sticky top-20 grid gap-2 text-sm">
              {titles.map((section) => (
                <li key={section.id}>
                  <a
                    href={`#${section.id}`}
                    className="text-muted-foreground hover:text-foreground"
                  >
                    {section.title}
                  </a>
                </li>
              ))}
            </ul>
          ) : null}
        </nav>
        <article ref={article} className="max-w-[68ch]">
          <h1 className="font-display font-wide text-4xl font-bold tracking-tight sm:text-5xl">
            {title}
          </h1>
          <p className="mt-4 text-muted-foreground">Last updated {updated}</p>
          <div className="mt-12 space-y-12">{children}</div>
        </article>
      </div>
    </PublicLayout>
  );
}

export function LegalSection({
  title,
  children,
}: {
  title: string;
  children: ReactNode;
}) {
  return (
    <section>
      <h2
        id={sectionId(title)}
        className="scroll-mt-20 font-display text-xl font-semibold tracking-tight"
      >
        {title}
      </h2>
      <div className="mt-3 space-y-3 text-[1.0625rem] leading-7 text-muted-foreground">
        {children}
      </div>
    </section>
  );
}

export function LegalSubsection({
  title,
  children,
}: {
  title: string;
  children: ReactNode;
}) {
  return (
    <div className="space-y-3 pt-2">
      <h3 className="font-medium text-foreground">{title}</h3>
      {children}
    </div>
  );
}

export function LegalList({ items }: { items: string[] }) {
  return (
    <ul className="list-disc space-y-1 pl-5">
      {items.map((item) => (
        <li key={item}>{item}</li>
      ))}
    </ul>
  );
}

export function ContactEmail() {
  return (
    <a
      href={`mailto:${CONTACT_EMAIL}`}
      className="text-foreground underline underline-offset-4"
    >
      {CONTACT_EMAIL}
    </a>
  );
}
