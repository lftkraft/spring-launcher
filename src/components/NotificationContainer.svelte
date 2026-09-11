<script lang="ts">
  import { notifications, removeNotification } from '../stores/notification';
  import Notification from './Notification.svelte';
</script>

<div class="notification-container">
  {#each $notifications as item (item.id)}
    <Notification
      message={item.message}
      type={item.type}
      duration={item.duration}
      onClose={() => removeNotification(item.id)}
    />
  {/each}
</div>
