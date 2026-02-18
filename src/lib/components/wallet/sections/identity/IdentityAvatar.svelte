<script lang="ts">
  type IdentityAvatarProps = {
    seed: string;
    label: string;
    class?: string;
  };

  const gradients = [
    ['#2563EB', '#0891B2'],
    ['#2563EB', '#4F46E5'],
    ['#0F766E', '#2563EB'],
    ['#1D4ED8', '#0E7490'],
    ['#0EA5E9', '#1D4ED8'],
    ['#1E3A8A', '#0284C7']
  ] as const;

  function hashSeed(value: string): number {
    let hash = 0;
    for (const char of value.trim().toLowerCase()) {
      hash = (hash * 31 + char.charCodeAt(0)) >>> 0;
    }
    return hash;
  }

  function deriveInitials(value: string): string {
    const normalized = value.replace(/@/g, '').trim();
    if (!normalized) return '@';

    const segments = normalized.split('.').filter(Boolean);
    if (segments.length >= 2) {
      return `${segments[0].slice(0, 1)}${segments[1].slice(0, 1)}`.toUpperCase();
    }

    return normalized.slice(0, 2).toUpperCase();
  }

  /* eslint-disable prefer-const */
  let { seed, label, class: className = '' }: IdentityAvatarProps = $props();
  /* eslint-enable prefer-const */

  const gradient = $derived(gradients[hashSeed(seed) % gradients.length]);
  const initials = $derived(deriveInitials(label));
</script>

<div
  class={`inline-flex size-9 shrink-0 items-center justify-center rounded-full text-[11px] font-semibold tracking-wide text-white ${className}`}
  style={`background-image: linear-gradient(135deg, ${gradient[0]}, ${gradient[1]});`}
  aria-hidden="true"
>
  {initials}
</div>
