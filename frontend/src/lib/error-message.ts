import { ApiError } from "./api";

const SERVER_ERROR = 500;

const BY_STATUS: Partial<Record<number, string>> = {
  400: "That was not accepted. Check the details and try again.",
  401: "Your session has ended. Log in again.",
  403: "You do not have access to this.",
  404: "This could not be found.",
  409: "That conflicts with something that already exists.",
};

/**
 * A sentence for a person about a failed request. The API's own error text is
 * written for developers and is never shown; `overrides` gives a call site its
 * own sentence for a status it understands.
 */
export function errorMessage(
  error: unknown,
  overrides?: Partial<Record<number, string>>,
): string {
  if (error instanceof ApiError) {
    const known = overrides?.[error.status] ?? BY_STATUS[error.status];
    if (known) return known;
    if (error.status >= SERVER_ERROR) {
      return "Something went wrong on our side. Try again in a moment.";
    }
  }
  // fetch rejects with a TypeError when the request never reached the server.
  if (error instanceof TypeError) {
    return "The server could not be reached. Check your connection and try again.";
  }
  return "Something went wrong. Try again.";
}
