import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import type { Booking } from "../../bindings/Booking";
import { api } from "../../lib/api";

export const Route = createFileRoute("/dashboard/bookings")({ component: BookingsPage });
function BookingsPage() {
  const [bookings, setBookings] = useState<Booking[]>([]);
  useEffect(() => {
    api<Booking[]>("/api/bookings")
      .then(setBookings)
      .catch(() => setBookings([]));
  }, []);
  return (
    <section>
      <h1>Bookings</h1>
      <div className="card">
        {bookings.length === 0 ? (
          <p>No bookings yet.</p>
        ) : (
          bookings.map((booking) => (
            <article
              key={booking.unid}
              style={{ borderBottom: "1px solid #eee", padding: "12px 0" }}
            >
              <strong>{booking.invitee_name}</strong>
              <span> · {new Date(booking.slot_start).toLocaleString()}</span>
              <small style={{ display: "block", color: "#737373" }}>
                {booking.invitee_email} · {booking.status}
              </small>
            </article>
          ))
        )}
      </div>
    </section>
  );
}
