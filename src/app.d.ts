// Updated: Restore standard Svelte type reference + rune globals.
/// <reference types="svelte" />

declare global {
  function $state<T>(initial: T): T;
  function $derived<T>(value: T): T;
  function $effect(run: () => void | (() => void)): void;
  function $props<T = Record<string, unknown>>(): T;
  function $bindable<T>(value: T): T;
}

export {};
