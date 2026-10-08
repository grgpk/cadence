import { createFileRoute } from "@tanstack/react-router";

import { LandingPage } from "../domains/landing/LandingPage";
import { APP_NAME } from "../lib/config";

const TITLE = `${APP_NAME} | One link for people to book you`;

export const Route = createFileRoute("/")({
  head: () => ({ meta: [{ title: TITLE }, { property: "og:title", content: TITLE }] }),
  component: LandingPage,
});
