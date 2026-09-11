<script lang="ts">
  import { onMount } from 'svelte';
  import { AlertCircle, CheckCircle, Info, X } from 'lucide-svelte';
  import type { NotificationType } from '../stores/notification';
  import './Notification.css';

  let { message, type = 'info', duration = 5000, onClose } = $props<{
    message: string;
    type: NotificationType;
    duration?: number;
    onClose: () => void;
  }>();

  let isClosing = $state(false);

  function handleClose() {
    isClosing = true;
    setTimeout(() => {
      onClose();
    }, 400);
  }

  onMount(() => {
    const timer = setTimeout(handleClose, duration);
    return () => clearTimeout(timer);
  });
</script>

<div class={`notification-item ${type} ${isClosing ? 'closing' : ''}`}>
  <div class="notification-icon">
    {#if type === 'error'}
      <AlertCircle size={20} />
    {:else if type === 'success'}
      <CheckCircle size={20} />
    {:else}
      <Info size={20} />
    {/if}
  </div>
  <div class="notification-content">
    <p>{message}</p>
  </div>
  <button class="notification-close" onclick={handleClose} aria-label="Close notification">
    <X size={16} />
  </button>
</div>
