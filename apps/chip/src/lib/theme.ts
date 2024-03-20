import { writable } from "svelte/store";

export const THEMES = ['light', 'dark'] as const;
export type Theme = typeof THEMES[number];
export const theme = writable<Theme>('dark');
