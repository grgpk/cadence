/** A Google Calendar style mark: a page with a blue binding and the day. */
export function GoogleCalendarIcon() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true" className="size-5 shrink-0">
      <rect
        x="2.5"
        y="3"
        width="19"
        height="18.5"
        rx="3"
        fill="#fff"
        stroke="#1a73e8"
        strokeWidth="1.5"
      />
      <path d="M2.5 6a3 3 0 0 1 3-3h13a3 3 0 0 1 3 3v2h-19z" fill="#1a73e8" />
      <text
        x="12"
        y="18.25"
        textAnchor="middle"
        fontSize="8.5"
        fontWeight="700"
        fontFamily="system-ui, sans-serif"
        fill="#1a73e8"
      >
        31
      </text>
    </svg>
  );
}
