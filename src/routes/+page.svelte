<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { goto } from '$app/navigation';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import {
    Play, Square, Flame, Sparkles, Clock, User, Folder, FolderOpen,
    Plus, Layers, Package, Download, ChevronRight,
    Loader2, Globe, Monitor, Users, Server, Box, Trash2, RefreshCw
  } from 'lucide-svelte';
  import type { Instance } from '../types/instance';
  import type { Profile } from '../types/profile';
  import type { ModrinthSearchHit } from '../types/mod';
  import type { RecentServer } from '../types/server';
  import { showNotification } from '../stores/notification';
  import CreateInstanceModal from '../components/CreateInstanceModal.svelte';
  import AddServerModal from '../components/AddServerModal.svelte';
  import './Dashboard.css';

  let instances = $state<Instance[]>([]);
  let profiles = $state<Profile[]>([]);
  let activeProfileId = $state<string | null>(null);
  let loading = $state(true);
  let showCreateModal = $state(false);

  // Recent Servers state
  let recentServers = $state<RecentServer[]>([]);
  let serversLoading = $state(false);
  let showAddServerModal = $state(false);
  let launchingServerId = $state<string | null>(null);

  // Trending state
  let trendingType = $state<'mod' | 'modpack'>('mod');
  let trendingMods = $state<ModrinthSearchHit[]>([]);
  let trendingModpacks = $state<ModrinthSearchHit[]>([]);
  let trendingLoading = $state(false);

  // Launch state
  let isLaunching = $state(false);
  let runningInstanceId = $state<string | null>(null);

  let unlistenUpdated: (() => void) | undefined;
  let unlistenStopped: (() => void) | undefined;

  let activeProfile = $derived(
    profiles.find((p) => p.id === activeProfileId) || (profiles.length > 0 ? profiles[0] : null)
  );

  let lastPlayedInstance = $derived.by(() => {
    if (instances.length === 0) return null;
    const sorted = [...instances].sort((a, b) => {
      const aTime = a.lastPlayed ? new Date(a.lastPlayed).getTime() : 0;
      const bTime = b.lastPlayed ? new Date(b.lastPlayed).getTime() : 0;
      if (bTime !== aTime) return bTime - aTime;
      return new Date(b.created).getTime() - new Date(a.created).getTime();
    });
    return sorted[0];
  });

  let otherInstances = $derived.by(() => {
    if (!lastPlayedInstance) return [];
    return instances.filter((i) => i.id !== lastPlayedInstance!.id).slice(0, 4);
  });

  let totalPlaytimeFormatted = $derived.by(() => {
    const totalSec = instances.reduce((acc, i) => acc + (i.playtime || 0), 0);
    if (totalSec >= 3600) {
      return `${(totalSec / 3600).toFixed(1)} óra`;
    } else if (totalSec >= 60) {
      return `${Math.floor(totalSec / 60)} perc`;
    } else if (totalSec > 0) {
      return `${totalSec} mp`;
    }
    return '0 óra';
  });

  function getGreeting(): { title: string; subtitle: string } {
    const hour = new Date().getHours();
    if (hour >= 5 && hour < 12) {
      return { title: 'Jó reggelt', subtitle: 'Készen állsz egy újabb kalandra?' };
    } else if (hour >= 12 && hour < 18) {
      return { title: 'Szép napot', subtitle: 'Folytasd a legutóbbi világodat vagy próbálj ki új modokat!' };
    } else {
      return { title: 'Kellemes estét', subtitle: 'Ideje leülni egy jó kis Minecraftozásra!' };
    }
  }

  let greeting = $derived(getGreeting());

  function formatRelativeDate(dateStr?: string): string {
    if (!dateStr) return 'Még nem játszottál vele';
    try {
      const date = new Date(dateStr);
      const now = new Date();
      const diffMs = now.getTime() - date.getTime();
      const diffMins = Math.floor(diffMs / (1000 * 60));
      const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
      const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

      if (diffMins < 5) return 'Épp most';
      if (diffMins < 60) return `${diffMins} perce`;
      if (diffHours < 24) return `${diffHours} órája`;
      if (diffDays === 1) return 'Tegnap';
      if (diffDays < 7) return `${diffDays} napja`;
      return date.toLocaleDateString('hu-HU', { month: 'short', day: 'numeric' });
    } catch {
      return 'Ismeretlen';
    }
  }

  function formatPlaytime(seconds?: number): string {
    if (!seconds) return '0 óra';
    if (seconds >= 3600) {
      return `${(seconds / 3600).toFixed(1)} óra`;
    }
    if (seconds >= 60) {
      return `${Math.floor(seconds / 60)} perc`;
    }
    return `${seconds} mp`;
  }

  function formatDownloads(count: number): string {
    if (count >= 1_000_000) {
      return `${(count / 1_000_000).toFixed(1)}M`;
    }
    if (count >= 1_000) {
      return `${(count / 1_000).toFixed(0)}k`;
    }
    return count.toString();
  }

  function getLoaderBadge(loader: string) {
    const l = loader.toLowerCase();
    switch (l) {
      case 'neoforge':
        return { name: 'NeoForge', icon: '/loaders/neoforge.svg', class: 'neoforge' };
      case 'forge':
        return { name: 'Forge', icon: '/loaders/forge.png', class: 'forge' };
      case 'fabric':
        return { name: 'Fabric', icon: '/loaders/fabric.svg', class: 'fabric' };
      case 'quilt':
        return { name: 'Quilt', icon: '/loaders/quilt.svg', class: 'quilt' };
      default:
        return { name: 'Vanilla', icon: '/loaders/vanilla.svg', class: 'vanilla' };
    }
  }

  async function loadInstances() {
    try {
      const data: any[] = await invoke('get_instances');
      instances = data.map((i) => ({
        id: i.id,
        name: i.name,
        mcVersion: i.mc_version || i.mcVersion,
        loader: i.loader,
        loaderVersion: i.loader_version || i.loaderVersion,
        memory: i.memory,
        javaPath: i.java_path || i.javaPath,
        gameDir: i.game_dir || i.gameDir,
        created: i.created,
        lastPlayed: i.last_played || i.lastPlayed,
        icon: i.icon,
        playtime: i.playtime || 0
      }));
      window.dispatchEvent(new Event('instances-loaded'));

      if (lastPlayedInstance) {
        checkRunning(lastPlayedInstance.id);
      }
    } catch (err) {
      console.error('Failed to load instances:', err);
    }
  }

  async function loadProfiles() {
    try {
      const data: Profile[] = await invoke('get_profiles');
      profiles = data;
      const storedId = localStorage.getItem('activeProfileId');
      if (storedId && data.some((p) => p.id === storedId)) {
        activeProfileId = storedId;
      } else if (data.length > 0) {
        activeProfileId = data[0].id;
        localStorage.setItem('activeProfileId', data[0].id);
      } else {
        activeProfileId = null;
      }
    } catch (err) {
      console.error('Failed to load profiles:', err);
    }
  }

  async function loadTrending() {
    trendingLoading = true;
    try {
      const [modsRes, packsRes]: [any, any] = await Promise.all([
        invoke('search_mods', {
          query: '',
          facets: JSON.stringify([['project_type:mod']]),
          limit: 6,
          offset: 0
        }),
        invoke('search_mods', {
          query: '',
          facets: JSON.stringify([['project_type:modpack']]),
          limit: 6,
          offset: 0
        })
      ]);
      trendingMods = modsRes.hits || [];
      trendingModpacks = packsRes.hits || [];
    } catch (err) {
      console.error('Failed to load trending items:', err);
    } finally {
      trendingLoading = false;
    }
  }

  async function checkRunning(id: string) {
    try {
      const isRunning: boolean = await invoke('is_instance_running', { id });
      if (isRunning) {
        runningInstanceId = id;
      } else if (runningInstanceId === id) {
        runningInstanceId = null;
      }
    } catch {
      runningInstanceId = null;
    }
  }

  async function handleLaunch(instance: Instance) {
    if (isLaunching) return;

    if (runningInstanceId === instance.id) {
      showNotification('Ez az instance jelenleg is fut!', 'info');
      return;
    }

    isLaunching = true;
    try {
      let profile = activeProfile;
      if (!profile && profiles.length > 0) {
        profile = profiles[0];
      }
      if (!profile) {
        showNotification('Nincs kiválasztott profil! Hozz létre egyet a Profilkezelőben.', 'error');
        goto('/profiles');
        return;
      }

      showNotification(`"${instance.name}" indítása...`, 'info');
      await invoke('launch_instance', {
        id: instance.id,
        profile
      });
      runningInstanceId = instance.id;
      showNotification(`"${instance.name}" elindult!`, 'success');
      await loadInstances();
    } catch (err) {
      console.error('Launch failed:', err);
      showNotification(`Indítási hiba: ${err}`, 'error');
    } finally {
      isLaunching = false;
    }
  }

  async function loadRecentServers() {
    try {
      serversLoading = true;
      const data: RecentServer[] = await invoke('get_recent_servers');
      recentServers = data;
    } catch (err) {
      console.error('Failed to load recent servers:', err);
    } finally {
      serversLoading = false;
    }
  }

  async function handleLaunchServer(server: RecentServer) {
    if (isLaunching) return;

    const inst = instances.find((i) => i.id === server.instance_id);
    if (!inst) {
      showNotification(`A(z) "${server.instance_name}" instance nem található!`, 'error');
      return;
    }

    if (runningInstanceId === inst.id) {
      showNotification('Ez az instance jelenleg is fut!', 'info');
      return;
    }

    isLaunching = true;
    launchingServerId = server.id;
    try {
      let profile = activeProfile;
      if (!profile && profiles.length > 0) {
        profile = profiles[0];
      }
      if (!profile) {
        showNotification('Nincs kiválasztott profil! Hozz létre egyet a Profilkezelőben.', 'error');
        goto('/profiles');
        return;
      }

      showNotification(`Csatlakozás: ${server.server_name} (${inst.name})...`, 'info');
      await invoke('launch_instance_server', {
        instanceId: inst.id,
        profile,
        serverAddress: server.server_address
      });
      runningInstanceId = inst.id;
      showNotification(`"${inst.name}" elindult! Csatlakozás: ${server.server_name}`, 'success');
      await Promise.all([loadInstances(), loadRecentServers()]);
    } catch (err) {
      console.error('Launch server failed:', err);
      showNotification(`Indítási hiba: ${err}`, 'error');
    } finally {
      isLaunching = false;
      launchingServerId = null;
    }
  }

  async function handleRemoveServer(id: string) {
    try {
      await invoke('remove_recent_server', { id });
      recentServers = recentServers.filter((s) => s.id !== id);
      showNotification('Szerver eltávolítva a listából.', 'info');
    } catch (err) {
      console.error('Failed to remove server:', err);
    }
  }

  onMount(async () => {
    loading = true;
    await Promise.all([loadInstances(), loadProfiles(), loadTrending(), loadRecentServers()]);
    loading = false;

    unlistenUpdated = await listen('instances-updated', () => {
      loadInstances();
      loadRecentServers();
    });

    unlistenStopped = await listen('instance-stopped', () => {
      runningInstanceId = null;
      loadInstances();
      loadRecentServers();
    });
  });

  onDestroy(() => {
    if (unlistenUpdated) unlistenUpdated();
    if (unlistenStopped) unlistenStopped();
  });
</script>

<div class="dashboard-noscroll-wrap">
  <div class="dashboard-container">
    {#if loading}
      <div class="dashboard-loading-state">
        <Loader2 class="spin" size={38} />
        <p>Dashboard betöltése...</p>
      </div>
    {:else}
      <!-- GREETING HEADER -->
      <div class="dash-welcome-bar">
        <div class="welcome-text-group">
          <span class="greeting-badge">
            <Sparkles size={14} />
            <span>{greeting.title}</span>
          </span>
          <h1 class="greeting-user">
            {#if activeProfile}
              {activeProfile.name}
            {:else}
              Játékos
            {/if}
          </h1>
          <p class="greeting-subtitle">{greeting.subtitle}</p>
        </div>
      </div>

      <!-- MAIN 2-COLUMN GRID -->
      <div class="dashboard-grid-layout">
        
        <!-- LEFT MAIN COLUMN -->
        <div class="dashboard-main-col">
          
          <!-- HERO FEATURED LAST PLAYED CARD -->
          <div class="dash-card hero-featured-card">
            <div class="card-top-label">
              <Clock size={15} />
              <span>UTOLJÁRA JÁTSZOTT PÉLDÁNY</span>
            </div>

            {#if lastPlayedInstance}
              {@const loaderBadge = getLoaderBadge(lastPlayedInstance.loader)}
              <div class="featured-instance-body">
                <div class="feat-icon-wrap">
                  {#if lastPlayedInstance.icon}
                    <img src={lastPlayedInstance.icon} alt="" class="feat-icon" />
                  {:else}
                    <img src={loaderBadge.icon} alt="" class="feat-icon loader-img" />
                  {/if}
                </div>

                <div class="feat-meta-wrap">
                  <div class="feat-badge-row">
                    <span class={`loader-badge ${loaderBadge.class}`}>{loaderBadge.name}</span>
                    <span class="meta-pill">MC {lastPlayedInstance.mcVersion}</span>
                    <span class="meta-pill">{lastPlayedInstance.memory} MB RAM</span>
                  </div>

                  <h2 class="feat-name truncate-text" title={lastPlayedInstance.name}>{lastPlayedInstance.name}</h2>

                  <div class="feat-time-row">
                    <span>Utoljára: {formatRelativeDate(lastPlayedInstance.lastPlayed)}</span>
                    <span class="dot-sep">•</span>
                    <span>Játékidő: {formatPlaytime(lastPlayedInstance.playtime)}</span>
                  </div>
                </div>

                <div class="feat-actions">
                  <button
                    type="button"
                    class={`hero-play-btn ${runningInstanceId === lastPlayedInstance.id ? 'running' : ''}`}
                    onclick={() => handleLaunch(lastPlayedInstance!)}
                    disabled={isLaunching}
                  >
                    {#if isLaunching}
                      <Loader2 class="spin" size={20} />
                      <span>Indítás...</span>
                    {:else if runningInstanceId === lastPlayedInstance.id}
                      <Square size={18} />
                      <span>Játékban</span>
                    {:else}
                      <Play size={20} fill="currentColor" />
                      <span>INDÍTÁS</span>
                    {/if}
                  </button>

                  <button
                    type="button"
                    class="hero-details-btn"
                    onclick={() => goto(`/instance/${lastPlayedInstance!.id}`)}
                    title="Instance megnyitása"
                  >
                    <FolderOpen size={17} />
                    <span>Részletek</span>
                  </button>
                </div>
              </div>
            {:else}
              <div class="featured-empty-state">
                <div class="empty-icon-ring"><Package size={32} /></div>
                <div class="empty-meta">
                  <h3>Még nincs létrehozott instance-ed</h3>
                  <p>Hozd létre az első Minecraft példányodat egyetlen kattintással!</p>
                </div>
                <button
                  type="button"
                  class="btn-gradient-green"
                  onclick={() => (showCreateModal = true)}
                >
                  <Plus size={16} />
                  <span>Új Instance Létrehozása</span>
                </button>
              </div>
            {/if}
          </div>

          <!-- TRENDING MODS / MODPACKS SECTION -->
          <div class="dash-card trending-card-container">
            <div class="dash-card-header">
              <div class="header-left">
                <Flame class="flame-icon" size={20} />
                <h3>Felkapott Kiegészítők</h3>
              </div>

              <div class="header-right">
                <div class="segmented-pill-toggle">
                  <button
                    type="button"
                    class={`seg-btn ${trendingType === 'mod' ? 'active' : ''}`}
                    onclick={() => (trendingType = 'mod')}
                  >
                    <Package size={13} />
                    <span>Modok</span>
                  </button>
                  <button
                    type="button"
                    class={`seg-btn ${trendingType === 'modpack' ? 'active' : ''}`}
                    onclick={() => (trendingType = 'modpack')}
                  >
                    <Layers size={13} />
                    <span>Modpackek</span>
                  </button>
                </div>

                <button
                  type="button"
                  class="text-link-btn"
                  onclick={() => goto('/mods')}
                >
                  <span>Összes</span>
                  <ChevronRight size={14} />
                </button>
              </div>
            </div>

            {#if trendingLoading}
              <div class="trending-cards-grid">
                {#each Array(6) as _}
                  <div class="tr-item skeleton">
                    <div class="sk-icon"></div>
                    <div class="sk-lines">
                      <div class="sk-line title"></div>
                      <div class="sk-line text"></div>
                    </div>
                  </div>
                {/each}
              </div>
            {:else}
              {@const items = trendingType === 'mod' ? trendingMods : trendingModpacks}
              <div class="trending-cards-grid">
                {#each items as item (item.project_id)}
                  <!-- svelte-ignore a11y_click_events_have_key_events -->
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <div
                    class="trending-item-card"
                    onclick={() => goto(`/mods?search=${encodeURIComponent(item.title)}`)}
                  >
                    <div class="item-icon-box">
                      {#if item.icon_url}
                        <img src={item.icon_url} alt="" />
                      {:else}
                        <Package size={24} />
                      {/if}
                    </div>
                    <div class="item-info">
                      <div class="item-title-row">
                        <span class="item-title truncate-text" title={item.title}>{item.title}</span>
                      </div>
                      <span class="item-author">{item.author}</span>
                      <p class="item-desc">{item.description}</p>
                      <div class="item-footer">
                        <span class="item-dls">
                          <Download size={12} />
                          <span>{formatDownloads(item.downloads)}</span>
                        </span>
                        <span class="item-open-hint">Böngészés →</span>
                      </div>
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          </div>

          <!-- RECENT SERVERS CARD -->
          <div class="dash-card servers-card-container">
            <div class="dash-card-header">
              <div class="header-left">
                <Server class="server-icon" size={20} />
                <h3>Legutóbb Játszott Szerverek</h3>
              </div>
              <div class="header-right">
                <button
                  type="button"
                  class="mini-icon-btn"
                  onclick={loadRecentServers}
                  title="Frissítés"
                  disabled={serversLoading}
                >
                  <RefreshCw size={13} class={serversLoading ? 'spin' : ''} />
                </button>
                <button
                  type="button"
                  class="btn-small-green"
                  onclick={() => (showAddServerModal = true)}
                >
                  <Plus size={14} />
                  <span>Új Szerver</span>
                </button>
              </div>
            </div>

            <div class="servers-grid-list">
              {#if serversLoading && recentServers.length === 0}
                <div class="srv-empty-state">
                  <Loader2 size={20} class="spin" />
                  <span>Szerverek betöltése...</span>
                </div>
              {:else if recentServers.length > 0}
                {#each recentServers as srv (srv.id)}
                  <div class="server-item-card">
                    <div class="srv-card-icon">
                      {#if srv.icon}
                        <img src={srv.icon.startsWith('data:') ? srv.icon : `data:image/png;base64,${srv.icon}`} alt="" class="srv-img" />
                      {:else}
                        <img
                          src={`https://api.mcstatus.io/v2/icon/${encodeURIComponent(srv.server_address)}`}
                          alt=""
                          class="srv-img"
                          onerror={(e) => { (e.currentTarget as HTMLElement).style.display = 'none'; }}
                        />
                        <Server size={28} class="srv-fallback-icon" />
                      {/if}
                    </div>

                    <div class="srv-card-details">
                      <span class="srv-name truncate-text" title={srv.server_name}>{srv.server_name}</span>
                      <span class="srv-addr truncate-text">{srv.server_address}</span>
                      <div class="srv-tags-row">
                        <span class="srv-inst-tag" title={`Példány: ${srv.instance_name}`}>
                          <Box size={13} />
                          <span class="srv-tag-name">{srv.instance_name}</span>
                        </span>
                        {#if srv.instance_loader}
                          {@const lBadge = getLoaderBadge(srv.instance_loader)}
                          <span class={`loader-badge ${lBadge.class}`}>{lBadge.name}</span>
                        {/if}
                        {#if srv.instance_version}
                          <span class="meta-pill">MC {srv.instance_version}</span>
                        {/if}
                      </div>
                    </div>

                    <div class="srv-card-actions">
                      <button
                        type="button"
                        class="srv-play-btn"
                        onclick={() => handleLaunchServer(srv)}
                        disabled={isLaunching}
                        title="Belépés"
                      >
                        {#if launchingServerId === srv.id}
                          <Loader2 size={16} class="spin" />
                        {:else}
                          <Play size={16} fill="currentColor" />
                        {/if}
                        <span>Belépés</span>
                      </button>
                      <button
                        type="button"
                        class="srv-del-btn"
                        onclick={() => handleRemoveServer(srv.id)}
                        title="Eltávolítás"
                      >
                        <Trash2 size={16} />
                      </button>
                    </div>
                  </div>
                {/each}
              {:else}
                <div class="srv-empty-state">
                  <Server size={22} />
                  <p>Még nem adtál hozzá elmentett szervert.</p>
                  <button type="button" class="mini-create-link" onclick={() => (showAddServerModal = true)}>
                    + Új szerver hozzáadása
                  </button>
                </div>
              {/if}
            </div>
          </div>

        </div>

        <!-- RIGHT SIDEBAR COLUMN -->
        <div class="dashboard-side-col">

          <!-- PROFILE CARD (WELL-DEFINED SEPARATE WIDGET) -->
          <div class="dash-card profile-widget-card">
            <div class="card-top-label">
              <User size={15} />
              <span>AKTÍV PROFIL</span>
            </div>

            <div class="profile-widget-body">
              <div class="profile-avatar-box">
                {#if activeProfile}
                  {#if activeProfile.skinUrl}
                    <img src={activeProfile.skinUrl} alt={activeProfile.name} class="p-avatar" />
                  {:else if activeProfile.profile_type === 'microsoft'}
                    <img
                      src={`https://mc-heads.net/avatar/${activeProfile.name}/64`}
                      alt={activeProfile.name}
                      class="p-avatar"
                      onerror={(e) => {
                        (e.currentTarget as HTMLElement).style.display = 'none';
                      }}
                    />
                  {:else}
                    <User size={28} class="avatar-icon" />
                  {/if}
                {:else}
                  <User size={28} class="avatar-icon" />
                {/if}
                <div class="p-online-dot"></div>
              </div>

              <div class="profile-widget-text">
                <span class="p-name truncate-text">
                  {activeProfile ? activeProfile.name : 'Nincs profil'}
                </span>
                <span class={`p-type-badge ${activeProfile?.profile_type || 'offline'}`}>
                  {#if activeProfile?.profile_type === 'microsoft'}
                    <Globe size={11} /> Microsoft
                  {:else}
                    <Monitor size={11} /> Offline
                  {/if}
                </span>
              </div>
            </div>

            <button
              type="button"
              class="profile-switch-action-btn"
              onclick={() => goto('/profiles')}
            >
              <Users size={14} />
              <span>Profilváltás / Fiókok</span>
              <ChevronRight size={14} />
            </button>
          </div>

          <!-- LAUNCHER STATS CARD (WELL-DEFINED SEPARATE WIDGET) -->
          <div class="dash-card stats-widget-card">
            <div class="stat-row-item">
              <span class="stat-k">Összes Instance</span>
              <span class="stat-v">{instances.length}</span>
            </div>
            <div class="stat-divider"></div>
            <div class="stat-row-item">
              <span class="stat-k">Játékidő</span>
              <span class="stat-v">{totalPlaytimeFormatted}</span>
            </div>
          </div>

          <!-- OTHER INSTANCES QUICK LIST -->
          <div class="dash-card other-instances-card">
            <div class="card-header-mini">
              <div class="mini-title-wrap">
                <Folder size={15} />
                <span>További Példányok</span>
              </div>
              <button
                type="button"
                class="mini-add-btn"
                onclick={() => (showCreateModal = true)}
                title="Új instance"
              >
                <Plus size={15} />
              </button>
            </div>

            {#if otherInstances.length > 0}
              <div class="mini-instances-list">
                {#each otherInstances as inst (inst.id)}
                  {@const lBadge = getLoaderBadge(inst.loader)}
                  <!-- svelte-ignore a11y_click_events_have_key_events -->
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <div
                    class="mini-inst-item"
                    onclick={() => goto(`/instance/${inst.id}`)}
                  >
                    <div class="mini-inst-icon-box">
                      {#if inst.icon}
                        <img src={inst.icon} alt="" />
                      {:else}
                        <img src={lBadge.icon} alt="" class="loader-fallback" />
                      {/if}
                    </div>

                    <div class="mini-inst-info">
                      <span class="mini-inst-title truncate-text">{inst.name}</span>
                      <span class="mini-inst-sub">{lBadge.name} • {inst.mcVersion}</span>
                    </div>

                    <button
                      type="button"
                      class="mini-launch-btn"
                      title="Indítás"
                      onclick={(e) => {
                        e.stopPropagation();
                        handleLaunch(inst);
                      }}
                      disabled={isLaunching}
                    >
                      {#if runningInstanceId === inst.id}
                        <Square size={13} class="stop-icon" />
                      {:else}
                        <Play size={13} fill="currentColor" />
                      {/if}
                    </button>
                  </div>
                {/each}
              </div>
            {:else if instances.length <= 1}
              <div class="mini-empty-wrap">
                <p>Nincs több instance.</p>
                <button
                  type="button"
                  class="mini-create-link"
                  onclick={() => (showCreateModal = true)}
                >
                  + Létrehozás
                </button>
              </div>
            {/if}

            <button
              type="button"
              class="all-instances-footer-btn"
              onclick={() => goto('/instances')}
            >
              <span>Minden instance megtekintése</span>
              <ChevronRight size={15} />
            </button>
          </div>

        </div>

      </div>

    {/if}
  </div>
</div>

{#if showCreateModal}
  <CreateInstanceModal
    onClose={() => (showCreateModal = false)}
    onCreated={async () => {
      await loadInstances();
      showCreateModal = false;
    }}
  />
{/if}

{#if showAddServerModal}
  <AddServerModal
    {instances}
    onClose={() => (showAddServerModal = false)}
    onAdded={async () => {
      await loadRecentServers();
      showAddServerModal = false;
    }}
  />
{/if}
