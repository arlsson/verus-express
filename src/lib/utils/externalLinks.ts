import { openUrl } from '@tauri-apps/plugin-opener';

export const COMMUNITY_HANGOUT_URL = 'https://discord.gg/VRSC';

export async function openExternalUrl(url: string): Promise<void> {
  try {
    await openUrl(url);
    return;
  } catch {
    // Fall through to browser API fallback.
  }

  if (typeof globalThis.open === 'function') {
    globalThis.open(url, '_blank', 'noopener,noreferrer');
  }
}

export async function openCommunityHangout(): Promise<void> {
  await openExternalUrl(COMMUNITY_HANGOUT_URL);
}
