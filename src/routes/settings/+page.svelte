<script lang="ts">
  import { goto } from '$app/navigation';
  import SettingsModal from '../../components/SettingsModal.svelte';

  function handleClose() {
    goto('/');
  }
</script>

<SettingsModal onClose={handleClose} />
