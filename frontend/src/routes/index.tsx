import { createFileRoute } from "@tanstack/react-router";
import { BookingWidget } from "../domains/widget/BookingWidget";
import { PUBLIC_HOST_UNID } from "../lib/config";

export const Route = createFileRoute("/")({ component: HomePage });

function HomePage() {
  return <main style={{ maxWidth: 960, margin: "0 auto", padding: 32 }}>
    <header style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 48 }}>
      <strong style={{ fontSize: 24 }}>Cadence</strong><a href="/login">Host login</a>
    </header>
    <section style={{ display: "grid", gap: 32, gridTemplateColumns: "1fr minmax(320px, 420px)", alignItems: "center" }}>
      <div><p style={{ color: "#737373" }}>Simple booking cadence.</p><h1 style={{ fontSize: 52, lineHeight: 1.05, margin: "12px 0" }}>Book the right call at the right time.</h1><p style={{ color: "#525252", fontSize: 18 }}>Cadence gives hosts a clean booking widget and a focused dashboard.</p></div>
      <BookingWidget hostUnid={PUBLIC_HOST_UNID} />
    </section>
  </main>;
}
