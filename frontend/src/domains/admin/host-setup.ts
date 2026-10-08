import type { AdminHost } from "../../bindings/AdminHost";

/** What still stops a host from taking calls. The labels double as search words. */
export const HOST_GAPS = {
  notStarted: "Not started",
  noHours: "No hours",
  noCallLink: "No call link",
} as const;

type HostGap = (typeof HOST_GAPS)[keyof typeof HOST_GAPS];

export function hostGaps(host: AdminHost): HostGap[] {
  if (host.public_slug === null) return [HOST_GAPS.notStarted];
  const gaps: HostGap[] = [];
  if (!host.has_hours) gaps.push(HOST_GAPS.noHours);
  if (!host.calendar_connected && !host.has_meeting_link) gaps.push(HOST_GAPS.noCallLink);
  return gaps;
}
