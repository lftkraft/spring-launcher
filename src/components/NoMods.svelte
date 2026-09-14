<script lang="ts">
  import { Ghost, Plus } from 'lucide-svelte';
  import { goto } from '$app/navigation';
  import { t } from '../stores/i18n';
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
      <h1>{$t('noMods.title')}</h1>
      <p>{$t('noMods.desc')}</p>
    </div>

    <div class="no-mods-actions">
      <button class="btn btn-primary" onclick={() => goto('/')}>
        <Plus size={18} />
        <span>{$t('noMods.createBtn')}</span>
      </button>
    </div>
  </div>

  <div class="background-elements">
    <div class="blob"></div>
    <div class="blob secondary"></div>
  </div>
  <div class="error-code">404</div>
</div>
