<script lang="ts">
  import { Ghost, Plus } from 'lucide-svelte';
  import { goto } from '$app/navigation';
  import './NoMods.css';
</script>

<div class="no-mods-container fade-in">
  <div class="no-mods-content">
    <div class="glitch-wrapper">
      <Ghost size={80} class="ghost-icon" />
      <div class="glitch-overlay"></div>
    </div>

    <div class="text-content">
      <h2 class="error-code">404</h2>
      <h1>Itt nincsenek modok</h1>
      <p>Úgy tűnik, egy Vanilla instance-et nézel. A vanilla verziók nem támogatják a modokat. Ha modolni szeretnél, hozz létre egy Fabric vagy Forge instance-et!</p>
    </div>

    <div class="no-mods-actions">
      <button class="btn btn-primary" onclick={() => goto('/')}>
        <Plus size={18} />
        <span>Új instance létrehozása</span>
      </button>
    </div>
  </div>

  <div class="background-elements">
    <div class="blob"></div>
    <div class="blob secondary"></div>
  </div>
  <div class="error-code">404</div>
</div>
