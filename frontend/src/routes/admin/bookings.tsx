import { createFileRoute } from "@tanstack/react-router";

import type { Booking } from "../../bindings/Booking";
import {
  AdminDataPage,
  formatAdminDate,
} from "../../domains/admin/components/admin-data-page";

export const Route = createFileRoute("/admin/bookings")({
  component: AdminBookingsPage,
});

function AdminBookingsPage() {
  return (
    <AdminDataPage<Booking>
      title="Bookings"
      description="Booked calls across all hosts."
      endpoint="/api/admin/bookings"
      emptyMessage="No bookings found."
      itemKey={(booking) => booking.unid}
      renderItem={(booking) => (
        <div className="flex flex-wrap items-center justify-between gap-3 rounded-lg border p-4">
          <div>
            <p className="font-medium">{booking.invitee_name}</p>
            <p className="text-sm text-muted-foreground">{booking.invitee_email}</p>
          </div>
          <div className="text-right text-sm text-muted-foreground">
            <p>{booking.status}</p>
            <p>{formatAdminDate(booking.slot_start)}</p>
          </div>
        </div>
      )}
    />
  );
}
