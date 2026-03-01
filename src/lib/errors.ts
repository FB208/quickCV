export const asErrorMessage = (error: unknown, fallback = "发生未知错误，请查看日志"): string => {
  if (error instanceof Error) {
    const message = error.message.trim();
    if (message) {
      return message;
    }
  }

  if (typeof error === "string") {
    const message = error.trim();
    if (message) {
      return message;
    }
  }

  if (typeof error === "object" && error !== null) {
    const payload = error as Record<string, unknown>;
    const fields = [payload.message, payload.error, payload.details];
    for (const field of fields) {
      if (typeof field === "string" && field.trim()) {
        return field.trim();
      }
      if (field instanceof Error && field.message.trim()) {
        return field.message.trim();
      }
    }

    try {
      const serialized = JSON.stringify(payload);
      if (serialized && serialized !== "{}") {
        return serialized;
      }
    } catch {
      // ignore
    }
  }

  return fallback;
};
