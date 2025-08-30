<script lang="ts">
	import { notificationStore } from '$stores/notifications';
	import NotificationToast from './NotificationToast.svelte';
</script>

<!-- Fixed position notification container -->
<div class="fixed top-4 right-4 z-50 space-y-3">
	{#each notificationStore.notifications as notification (notification.id)}
		<NotificationToast {notification} />
	{/each}
</div>