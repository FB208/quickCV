export const formatHotkey = (event: KeyboardEvent): string => {
  const parts: string[] = [];

  if (event.ctrlKey) {
    parts.push("Ctrl");
  }
  if (event.altKey) {
    parts.push("Alt");
  }
  if (event.shiftKey) {
    parts.push("Shift");
  }
  if (event.metaKey) {
    parts.push("Meta");
  }

  const key = normalizeKey(event.key);
  if (key && !isModifierOnlyKey(key)) {
    parts.push(key);
  }

  return parts.join("+");
};

const normalizeKey = (key: string): string => {
  const normalized = key.trim();
  if (!normalized) {
    return "";
  }

  if (normalized.length === 1) {
    return normalized.toUpperCase();
  }

  const mapping: Record<string, string> = {
    Escape: "Esc",
    " ": "Space",
    ArrowUp: "Up",
    ArrowDown: "Down",
    ArrowLeft: "Left",
    ArrowRight: "Right"
  };

  return mapping[normalized] || normalized;
};

const isModifierOnlyKey = (key: string): boolean => {
  return key === "Control" || key === "Alt" || key === "Shift" || key === "Meta";
};
