// Svelte 5 notification system using runes

export interface Notification {
	id: string;
	type: 'success' | 'error' | 'warning' | 'info';
	title: string;
	message?: string;
	duration?: number;
	timestamp: number;
}

// Svelte 5 reactive state
let notifications = $state<Notification[]>([]);

export const notificationStore = {
	// Reactive getter
	get notifications() { 
		return notifications; 
	},

	// Add notification
	add(notification: Omit<Notification, 'id' | 'timestamp'>) {
		const newNotification: Notification = {
			...notification,
			id: crypto.randomUUID(),
			timestamp: Date.now(),
			duration: notification.duration ?? 5000
		};

		notifications = [newNotification, ...notifications];

		// Auto-remove after duration
		if (newNotification.duration > 0) {
			setTimeout(() => {
				notificationStore.remove(newNotification.id);
			}, newNotification.duration);
		}

		return newNotification.id;
	},

	// Remove notification
	remove(id: string) {
		notifications = notifications.filter(n => n.id !== id);
	},

	// Clear all notifications
	clear() {
		notifications = [];
	},

	// Convenience methods
	success(title: string, message?: string, duration?: number) {
		return this.add({ type: 'success', title, message, duration });
	},

	error(title: string, message?: string, duration?: number) {
		return this.add({ type: 'error', title, message, duration: duration ?? 8000 });
	},

	warning(title: string, message?: string, duration?: number) {
		return this.add({ type: 'warning', title, message, duration });
	},

	info(title: string, message?: string, duration?: number) {
		return this.add({ type: 'info', title, message, duration });
	}
};