export function getErrorMessage(
  error: unknown,
  fallback: string,
): string {
  if (error instanceof Error && error.message) {
    return error.message;
  }

  if (typeof error === "string" && error.trim()) {
    return error;
  }

  if (error && typeof error === "object") {
    const message = Reflect.get(error, "message");
    if (typeof message === "string" && message.trim()) {
      return message;
    }

    const cause = Reflect.get(error, "cause");
    if (typeof cause === "string" && cause.trim()) {
      return cause;
    }

    const detail = Reflect.get(error, "error");
    if (typeof detail === "string" && detail.trim()) {
      return detail;
    }
  }

  return fallback;
}
