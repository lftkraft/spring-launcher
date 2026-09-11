import { writable } from 'svelte/store';
import type { Update } from '@tauri-apps/plugin-updater';

export const availableUpdate = writable<Update | null>(null);
export const showUpdaterModal = writable<boolean>(false);
export const isCheckingUpdate = writable<boolean>(false);
