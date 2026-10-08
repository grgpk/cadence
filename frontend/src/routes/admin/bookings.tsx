import { createFileRoute } from "@tanstack/react-router";

import type { Booking } from "../../bindings/Booking";
import { AdminDataPage } from "../../domains/admin/components/admin-data-page";
import { ADMIN_PAGES } from "../../domains/admin/routes";
import { bookingColumns } from "../../domains/bookings/booking-columns";
import { APP_NAME } from "../../lib/config";

const COLUMNS = bookingColumns();

export const Route = createFileRoute("/admin/bookings")({
  head: () => ({ meta: [{ title: `${ADMIN_PAGES.bookings.title} | ${APP_NAME}` }] }),
  component: AdminBookingsPage,
});

function AdminBookingsPage() {
  return (
    <AdminDataPage<Booking>
      {...ADMIN_PAGES.bookings}
      emptyTitle="No bookings yet"
      emptyDescription="Calls booked with any host are listed here."
      rowKey={(booking) => booking.unid}
      columns={COLUMNS}
    />
  );
}
