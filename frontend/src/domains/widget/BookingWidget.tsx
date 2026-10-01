import { useEffect, useState } from "react";
import type { TimeSlot } from "../../bindings/TimeSlot";
import { api } from "../../lib/api";

type Props = { hostUnid: string };

export function BookingWidget({ hostUnid }: Props) {
  const [slots, setSlots] = useState<TimeSlot[]>([]);
  const [slot, setSlot] = useState("");
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [message, setMessage] = useState("");
  useEffect(() => {
    if (hostUnid)
      api<TimeSlot[]>(`/api/hosts/${hostUnid}/slots?days=14`)
        .then(setSlots)
        .catch(() => setSlots([]));
  }, [hostUnid]);
  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    setMessage("");
    try {
      await api(`/api/hosts/${hostUnid}/bookings`, {
        method: "POST",
        body: JSON.stringify({
          slot_start: slot,
          timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
          invitee_name: name,
          invitee_email: email,
        }),
      });
      setMessage("Call booked. Check your email.");
      setSlot("");
    } catch (reason) {
      setMessage(reason instanceof Error ? reason.message : "Booking failed");
    }
  };
  if (!hostUnid)
    return (
      <div className="card">
        <h2>Booking widget</h2>
        <p>Set VITE_PUBLIC_HOST_UNID to activate public booking.</p>
      </div>
    );
  return (
    <div className="card">
      <h2>Book a call</h2>
      <form onSubmit={submit} style={{ display: "grid", gap: 14 }}>
        <label>
          Available slot
          <select
            className="input"
            value={slot}
            onChange={(e) => setSlot(e.target.value)}
            required
          >
            <option value="">Choose a time</option>
            {slots
              .filter((item) => item.available)
              .map((item) => (
                <option key={item.start} value={item.start}>
                  {new Date(item.start).toLocaleString()}
                </option>
              ))}
          </select>
        </label>
        <label>
          Name
          <input
            className="input"
            value={name}
            onChange={(e) => setName(e.target.value)}
            required
          />
        </label>
        <label>
          Email
          <input
            className="input"
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            required
          />
        </label>
        <button className="button" disabled={!slots.length}>
          Book call
        </button>
        {message && <p>{message}</p>}
      </form>
    </div>
  );
}
