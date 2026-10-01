/**
 * Server-side API origin. In production, set API_URL to backend's internal URL.
 * Browser requests use VITE_API_URL instead.
 */
export const SERVER_API_URL = process.env.API_URL ?? "http://localhost:3001";
export const CLIENT_API_URL = import.meta.env.VITE_API_URL ?? "http://localhost:3001";
