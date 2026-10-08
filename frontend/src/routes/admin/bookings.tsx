import { createFileRoute } from "@tanstack/react-router";

import type { Booking } from "../../bindings/Booking";
import { AdminDataPage } from "../../domains/admin/components/admin-data-page";
import { ADMIN_PAGES } from "../../domains/admin/routes";
import { BOOKING_COLUMNS } from "../../domains/bookings/booking-columns";

export const Route = createFileRoute("/admin/bookings")({
  component: AdminBookingsPage,
});

function AdminBookingsPage() {
  return (
    <AdminDataPage<Booking>
      {...ADMIN_PAGES.bookings}
      endpoint="/api/admin/bookings"
      emptyTitle="No bookings yet"
      emptyDescription="Calls booked with any host are listed here."
      rowKey={(booking) => booking.unid}
      columns={BOOKING_COLUMNS}
    />
  );
}
