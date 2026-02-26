export const ALLOWED_AUTO_LOCK_MINUTES = [5, 15, 30, 60] as const;
export type AutoLockMinutes = (typeof ALLOWED_AUTO_LOCK_MINUTES)[number];
export const DEFAULT_AUTO_LOCK_MINUTES: AutoLockMinutes = 15;

export function normalizeAutoLockMinutes(value: unknown): AutoLockMinutes {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return DEFAULT_AUTO_LOCK_MINUTES;
  if (!Number.isInteger(parsed)) return DEFAULT_AUTO_LOCK_MINUTES;

  const normalized = parsed as AutoLockMinutes;
  return ALLOWED_AUTO_LOCK_MINUTES.includes(normalized)
    ? normalized
    : DEFAULT_AUTO_LOCK_MINUTES;
}
