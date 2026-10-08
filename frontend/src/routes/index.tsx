import { createFileRoute } from "@tanstack/react-router";

import { LandingPage } from "../domains/landing/LandingPage";

export const Route = createFileRoute("/")({ component: LandingPage });
