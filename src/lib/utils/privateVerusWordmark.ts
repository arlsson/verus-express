const PRIVATE_BADGE_PATTERN = /\bPRIVATE\b/i;

export type PrivateVerusWordmarkParts = {
  leading: string;
  badge: string;
  trailing: string;
  hasBadge: boolean;
};

export function splitPrivateVerusWordmark(label: string): PrivateVerusWordmarkParts {
  const normalizedLabel = label.trim();
  if (!normalizedLabel) {
    return {
      leading: '',
      badge: '',
      trailing: '',
      hasBadge: false
    };
  }

  const match = PRIVATE_BADGE_PATTERN.exec(normalizedLabel);
  if (!match || match.index === undefined) {
    return {
      leading: normalizedLabel,
      badge: '',
      trailing: '',
      hasBadge: false
    };
  }

  const badgeStart = match.index;
  const badgeEnd = badgeStart + match[0].length;
  return {
    leading: normalizedLabel.slice(0, badgeStart).trimEnd(),
    badge: normalizedLabel.slice(badgeStart, badgeEnd).toUpperCase(),
    trailing: normalizedLabel.slice(badgeEnd).trimStart(),
    hasBadge: true
  };
}
