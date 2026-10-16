import { Link } from "@tanstack/react-router";

import { ROUTES } from "../../lib/routes";

const LINK_CLASS = "underline underline-offset-4 hover:text-foreground";

export function ConsentNotice() {
  return (
    <p className="text-xs leading-5 text-muted-foreground">
      By entering your information, you agree that your data will be stored in accordance
      with our{" "}
      <Link to={ROUTES.terms} className={LINK_CLASS}>
        terms
      </Link>{" "}
      &amp;{" "}
      <Link to={ROUTES.privacy} className={LINK_CLASS}>
        privacy policy
      </Link>
      .
    </p>
  );
}
