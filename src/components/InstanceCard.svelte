<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import type { Instance } from '../types/instance';
  import type { Profile } from '../types/profile';
  import './InstanceCard.css';

  let { instance, onDelete, onClick } = $props<{
    instance: Instance;
    onDelete: () => void;
    onClick: () => void;
  }>();

  let isLaunching = $state(false);
  let launchError = $state<string | null>(null);

  function getLoaderInfo(loader: string) {
    const map: Record<string, { icon: string; color: string }> = {
      vanilla: { icon: '⛏️', color: '#8BC34A' },
      fabric: { icon: '🧵', color: '#E91E63' },
      forge: { icon: '🔨', color: '#FF9800' },
      quilt: { icon: '🧶', color: '#2196F3' },
      neoforge: { icon: '⚡', color: '#9C27B0' }
    };
    return map[loader] || { icon: '📦', color: '#666' };
  }

  let loaderInfo = $derived(getLoaderInfo(instance.loader));

  async function handleLaunch(e: MouseEvent) {
    e.stopPropagation();
    isLaunching = true;
    launchError = null;

    try {
      const activeProfileId = localStorage.getItem('activeProfileId');
      if (!activeProfileId) {
        throw new Error('Nincs kiválasztott profil. Menj a Profilkezelőbe és válassz egyet.');
      }

      const profiles: Profile[] = await invoke('get_profiles');
      const profile = profiles.find((p) => p.id === activeProfileId);
      if (!profile) {
        throw new Error('A kiválasztott profil nem található. Ellenőrizd a Profilkezelőt.');
      }

      await invoke('launch_instance', {
        id: instance.id,
        profile
      });
    } catch (err) {
      console.error(`[InstanceCard][${instance.id}] launch-error`, err);
      launchError = `Indítási hiba: ${err}`;
      isLaunching = false;
    }
  }

  function handleDelete(e: MouseEvent) {
    e.stopPropagation();
    onDelete();
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="instance-card" onclick={onClick}>
  <div class="card-header">
    <div class="card-icon" style={`background: ${loaderInfo.color}`}>
      {loaderInfo.icon}
    </div>
    <div class="card-title">
      <h3>{instance.name}</h3>
      <span class="card-loader">
        {instance.loader.toUpperCase()} {instance.mcVersion}
      </span>
    </div>
  </div>

  <div class="card-body">
    <div class="card-info">
      <span>🧠 {instance.memory} MB</span>
      <span>📅 {new Date(instance.created).toLocaleDateString('hu-HU')}</span>
    </div>
    {#if instance.lastPlayed}
      <div class="card-last-played">
        Utoljára: {new Date(instance.lastPlayed).toLocaleString('hu-HU')}
      </div>
    {/if}
  </div>

  <div class="card-actions">
    <button class="btn-launch" onclick={handleLaunch} disabled={isLaunching}>
      {isLaunching ? '⏳ Indítás...' : '▶️ Indítás'}
    </button>
    <button class="btn-delete" onclick={handleDelete} title="Törlés">
      🗑️
    </button>
  </div>

  {#if launchError}
    <div class="card-error">{launchError}</div>
  {/if}
</div>
