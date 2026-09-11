import { writable } from 'svelte/store';

export type NotificationType = 'error' | 'success' | 'info';

export interface NotificationItem {
  id: string;
  message: string;
  type: NotificationType;
  duration?: number;
}

export const notifications = writable<NotificationItem[]>([]);

export function showNotification(message: string, type: NotificationType = 'info', duration: number = 5000) {
  const id = Math.random().toString(36).substring(2, 9);
  notifications.update((items) => [...items, { id, message, type, duration }]);
}

export function removeNotification(id: string) {
  notifications.update((items) => items.filter((n) => n.id !== id));
}
