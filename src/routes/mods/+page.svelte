<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import {
    Search, Download, Box, ChevronRight, ChevronLeft, Check, X, Loader2, AlertCircle,
    User, List, Sparkles, Zap, Wrench, Sword, Home, Wand2, LayoutGrid,
    Library, Cpu, Settings as SettingsIcon, Skull, Coins, Utensils,
    Puzzle, Ghost, Users2, Package, Truck, Mountain, Info, Folder,
    Filter, Layers, RotateCcw, ChevronDown, Plus, CheckCircle2, Clock,
    Palette, Database
  } from 'lucide-svelte';
  import type { ModrinthSearchHit, ModrinthVersion, DependencyInfo } from '../../types/mod';
  import type { Instance } from '../../types/instance';
  import { showNotification } from '../../stores/notification';
  import { portal } from '../../utils/portal';
  import './ModBrowser.css';

  const CATEGORIES = [
    { id: 'all', name: 'Összes', icon: LayoutGrid },
    { id: 'optimization', name: 'Optimization', icon: Zap },
    { id: 'utility', name: 'Utility', icon: Wrench },
    { id: 'adventure', name: 'Adventure', icon: Sword },
    { id: 'cursed', name: 'Cursed', icon: Skull },
    { id: 'decoration', name: 'Decoration', icon: Home },
    { id: 'economy', name: 'Economy', icon: Coins },
    { id: 'equipment', name: 'Equipment', icon: Sword },
    { id: 'food', name: 'Food', icon: Utensils },
    { id: 'game-mechanics', name: 'Game Mechanics', icon: Puzzle },
    { id: 'library', name: 'Library', icon: Library },
    { id: 'magic', name: 'Magic', icon: Wand2 },
    { id: 'management', name: 'Management', icon: SettingsIcon },
    { id: 'minigame', name: 'Minigame', icon: Puzzle },
    { id: 'mobs', name: 'Mobs', icon: Ghost },
    { id: 'social', name: 'Social', icon: Users2 },
    { id: 'storage', name: 'Storage', icon: Package },
    { id: 'technology', name: 'Technology', icon: Cpu },
    { id: 'transportation', name: 'Transportation', icon: Truck },
    { id: 'world-generation', name: 'World Gen', icon: Mountain }
  ];

  const TECHNICAL_TAGS = ['fabric', 'forge', 'quilt', 'neoforge', 'liteloader', 'modloader', 'rift'];

  const LOADERS = [
    { id: 'all', name: 'Összes loader', icon: null },
    { id: 'neoforge', name: 'NeoForge', icon: '/loaders/neoforge.svg' },
    { id: 'forge', name: 'Forge', icon: '/loaders/forge.png' },
    { id: 'fabric', name: 'Fabric', icon: '/loaders/fabric.svg' },
    { id: 'quilt', name: 'Quilt', icon: '/loaders/quilt.svg' }
  ];

  const POPULAR_MC_VERSIONS = [
    '1.21.4', '1.21.3', '1.21.1', '1.21',
    '1.20.6', '1.20.4', '1.20.2', '1.20.1', '1.20',
    '1.19.4', '1.19.2', '1.18.2', '1.17.1', '1.16.5', '1.12.2'
  ];

  let searchQuery = $state('');
  let activeCategory = $state('all');
  let filterLoader = $state('all');
  let filterVersion = $state('all');
  let showLoaderDropdown = $state(false);
  let showVersionDropdown = $state(false);
  let versionSearchQuery = $state('');
  let mcVersionsList = $state<string[]>([]);

  let selectedLoaderObj = $derived(LOADERS.find((l) => l.id === filterLoader) || LOADERS[0]);

  let availableVersions = $derived(
    mcVersionsList.length > 0 ? mcVersionsList : POPULAR_MC_VERSIONS
  );

  let filteredVersions = $derived(
    versionSearchQuery.trim()
      ? availableVersions.filter((v) => v.toLowerCase().includes(versionSearchQuery.trim().toLowerCase()))
      : availableVersions
  );

  let projectType = $state<'mod' | 'modpack' | 'resourcepack' | 'datapack'>('mod');
  let mods = $state<ModrinthSearchHit[]>([]);
  let loading = $state(false);
  let selectedMod = $state<ModrinthSearchHit | null>(null);
  let allVersions = $state<ModrinthVersion[]>([]);
  let instances = $state<Instance[]>([]);
  let installing = $state(false);

  let showInfoModal = $state(false);
  let showInstancePicker = $state(false);
  let showVersionPicker = $state(false);
  let showDepModal = $state(false);
  let showModpackNewModal = $state(false);
  let showModpackExistingModal = $state(false);
  let showModpackProgressModal = $state(false);

  let modpackInstanceName = $state('');
  let modpackMemory = $state(4096);
  let modpackTargetExisting = $state<Instance | null>(null);

  interface ModpackProgressState {
    step: string;
    current: number;
    total: number;
    file_name: string;
    percent: number;
    speed: number;
    total_bytes: number;
    current_bytes: number;
    remaining_time: string;
  }

  let modpackProgress = $state<ModpackProgressState>({
    step: '',
    current: 0,
    total: 0,
    file_name: '',
    percent: 0,
    speed: 0,
    total_bytes: 0,
    current_bytes: 0,
    remaining_time: ''
  });

  let targetInstance = $state<Instance | null>(null);
  let selectedVersion = $state<ModrinthVersion | null>(null);

  let isClosingModal = $state(false);
  let isClosingSubModal = $state(false);

  let dependencies = $state<DependencyInfo[]>([]);
  let selectedOptionalDeps = $state<string[]>([]);
  let scrollContainer = $state<HTMLDivElement | null>(null);
  let hoveredCategory = $state<{ name: string; y: number; right: number } | null>(null);

  // PAGINATION STATE
  let currentPage = $state(1);
  const pageSize = 24;
  let totalHits = $state(0);
  let totalPages = $derived(Math.max(1, Math.ceil(totalHits / pageSize)));

  let visiblePages = $derived.by(() => {
    const total = totalPages;
    const current = currentPage;
    if (total <= 7) {
      return Array.from({ length: total }, (_, i) => i + 1);
    }
    if (current <= 4) {
      return [1, 2, 3, 4, 5, '...', total];
    }
    if (current >= total - 3) {
      return [1, '...', total - 4, total - 3, total - 2, total - 1, total];
    }
    return [1, '...', current - 1, current, current + 1, '...', total];
  });

  function goToPage(page: number) {
    if (page < 1 || page > totalPages || page === currentPage || loading) return;
    handleSearch(page);
    if (scrollContainer) {
      scrollContainer.scrollTo({ top: 0, behavior: 'smooth' });
    }
  }

  function handleCategoryMouseEnter(e: MouseEvent, name: string) {
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    hoveredCategory = {
      name,
      y: rect.top + rect.height / 2,
      right: window.innerWidth - rect.left + 12
    };
  }

  function handleCategoryMouseLeave() {
    hoveredCategory = null;
  }

  let preferredInstanceId = $state<string | null>(null);
  let searchInitialized = false;
  let prevUrlSearch = '';

  let unlistenModpackProgress: (() => void) | null = null;

  onMount(async () => {
    await loadInstances();
    loadMcVersions();

    unlistenModpackProgress = await listen<any>('modpack-install-progress', (event) => {
      modpackProgress = event.payload;
    });

    const urlLoader = $page.url.searchParams.get('loader');
    const urlVersion = $page.url.searchParams.get('version');
    const urlInstanceId = $page.url.searchParams.get('instance');

    if (urlLoader && LOADERS.some((l) => l.id.toLowerCase() === urlLoader.toLowerCase())) {
      filterLoader = urlLoader.toLowerCase();
    }
    if (urlVersion) {
      filterVersion = urlVersion;
    }
    if (urlInstanceId) {
      preferredInstanceId = urlInstanceId;
      const foundInst = instances.find((i) => i.id === urlInstanceId);
      if (foundInst) {
        targetInstance = foundInst;
      }
    }

    handleSearch(1);
  });

  onDestroy(() => {
    if (unlistenModpackProgress) {
      unlistenModpackProgress();
    }
  });

  $effect(() => {
    const currentSearch = $page.url.search;
    if (currentSearch !== prevUrlSearch) {
      prevUrlSearch = currentSearch;
      const urlLoader = $page.url.searchParams.get('loader');
      const urlVersion = $page.url.searchParams.get('version');
      const urlInstanceId = $page.url.searchParams.get('instance');

      let filterChanged = false;
      if (urlLoader && LOADERS.some((l) => l.id.toLowerCase() === urlLoader.toLowerCase())) {
        if (filterLoader !== urlLoader.toLowerCase()) {
          filterLoader = urlLoader.toLowerCase();
          filterChanged = true;
        }
      }
      if (urlVersion && filterVersion !== urlVersion) {
        filterVersion = urlVersion;
        filterChanged = true;
      }
      if (urlInstanceId) {
        preferredInstanceId = urlInstanceId;
        const foundInst = instances.find((i) => i.id === urlInstanceId);
        if (foundInst) {
          targetInstance = foundInst;
        }
      }

      if (filterChanged && searchInitialized) {
        handleSearch(1);
      }
    }
  });

  $effect(() => {
    const cat = activeCategory;
    const ldr = filterLoader;
    const ver = filterVersion;
    if (!searchInitialized) {
      searchInitialized = true;
      return;
    }
    handleSearch(1);
  });

  $effect(() => {
    if (showInfoModal || showInstancePicker || showVersionPicker || showDepModal) {
      document.body.classList.add('modal-open');
    } else {
      document.body.classList.remove('modal-open');
    }
    return () => {
      document.body.classList.remove('modal-open');
    };
  });

  async function loadMcVersions() {
    try {
      const vers: string[] = await invoke('get_minecraft_versions');
      if (vers && vers.length > 0) {
        mcVersionsList = vers;
      }
    } catch (err) {
      console.error('Failed to load mc versions:', err);
    }
  }

  async function loadInstances() {
    try {
      const data: any[] = await invoke('get_instances');
      const mapped: Instance[] = data.map((i) => ({
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
      instances = mapped;
    } catch (err) {
      console.error(err);
    }
  }

  function selectLoader(id: string) {
    filterLoader = id;
    showLoaderDropdown = false;
    handleSearch(1);
  }

  function selectFilterVersion(ver: string) {
    filterVersion = ver;
    showVersionDropdown = false;
    handleSearch(1);
  }

  function selectCategory(catId: string) {
    activeCategory = catId;
    handleSearch(1);
  }

  function clearFilters() {
    filterLoader = 'all';
    filterVersion = 'all';
    activeCategory = 'all';
    searchQuery = '';
    versionSearchQuery = '';
    handleSearch(1);
  }

  function handleWindowClick(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (!target.closest('.filter-dropdown-wrapper')) {
      showLoaderDropdown = false;
      showVersionDropdown = false;
    }
  }

  function switchProjectType(type: 'mod' | 'modpack' | 'resourcepack' | 'datapack') {
    if (projectType === type) return;
    projectType = type;
    handleSearch(1);
  }

  async function handleSearch(page = 1) {
    loading = true;
    currentPage = page;
    try {
      let facetArray: string[][] = [[`project_type:${projectType}`]];
      if (activeCategory !== 'all') {
        facetArray.push([`categories:${activeCategory}`]);
      }
      if (filterLoader !== 'all' && projectType !== 'resourcepack' && projectType !== 'datapack') {
        facetArray.push([`categories:${filterLoader}`]);
      }
      if (filterVersion !== 'all') {
        facetArray.push([`versions:${filterVersion}`]);
      }
      const facets = JSON.stringify(facetArray);
      const offset = (page - 1) * pageSize;
      const result: any = await invoke('search_mods', {
        query: searchQuery,
        facets,
        limit: pageSize,
        offset
      });
      mods = result.hits || [];
      totalHits = result.total_hits || 0;
    } catch (err) {
      showNotification(`Hiba a keresés során: ${err}`, 'error');
    } finally {
      loading = false;
    }
  }

  async function openModInfo(mod: ModrinthSearchHit) {
    selectedMod = mod;
    selectedVersion = null;
    showInfoModal = true;
    isClosingModal = false;

    try {
      const versions: ModrinthVersion[] = await invoke('get_mod_versions', {
        projectId: mod.project_id,
        loader: null,
        gameVersion: null
      });
      allVersions = versions;

      const isContentPack =
        mod.project_type === 'resourcepack' ||
        mod.project_type === 'datapack' ||
        projectType === 'resourcepack' ||
        projectType === 'datapack';

      let compatible: Instance[];
      if (isContentPack) {
        // Resource packs and datapacks work on any instance (including vanilla)
        compatible = instances.filter((inst) =>
          versions.some((v) => v.game_versions.includes(inst.mcVersion))
        );
      } else {
        const moddable = instances.filter((i) => i.loader.toLowerCase() !== 'vanilla');
        compatible = moddable.filter((inst) =>
          versions.some(
            (v) =>
              v.game_versions.includes(inst.mcVersion) &&
              v.loaders.some((l) => l.toLowerCase() === inst.loader.toLowerCase())
          )
        );
      }

      const pool =
        compatible.length > 0
          ? compatible
          : isContentPack
            ? instances
            : instances.filter((i) => i.loader.toLowerCase() !== 'vanilla');

      let best: Instance | null = null;
      if (preferredInstanceId) {
        best = pool.find((i) => i.id === preferredInstanceId) || null;
      }
      if (!best) {
        best =
          pool.length > 0
            ? [...pool].sort((a, b) => (b.playtime || 0) - (a.playtime || 0))[0]
            : null;
      }
      targetInstance = best;

      if (mod.project_type === 'modpack' || projectType === 'modpack') {
        selectedVersion = versions.length > 0 ? versions[0] : null;
        modpackInstanceName = mod.title;
        modpackMemory = 4096;
        modpackTargetExisting = instances.length > 0 ? instances[0] : null;
      } else if (isContentPack) {
        if (best) {
          const bestVer = versions.find((v) => v.game_versions.includes(best!.mcVersion));
          selectedVersion = bestVer || versions[0] || null;
        } else {
          selectedVersion = versions.length > 0 ? versions[0] : null;
        }
      } else if (best) {
        const bestVer = versions.filter(
          (v) =>
            v.game_versions.includes(best!.mcVersion) &&
            v.loaders.some((l) => l.toLowerCase() === best!.loader.toLowerCase())
        )[0];
        selectedVersion = bestVer || null;
      }
    } catch (err) {
      console.error(err);
    }
  }

  async function handleInstallModpackNew() {
    if (!selectedMod || !selectedVersion) return;
    showModpackNewModal = false;
    showModpackProgressModal = true;
    modpackProgress = { step: 'Előkészítés...', current: 0, total: 100, file_name: '', percent: 1, speed: 0, total_bytes: 0, current_bytes: 0, remaining_time: '' };

    try {
      const inst: any = await invoke('install_modpack_new_instance', {
        versionId: selectedVersion.id,
        instanceName: modpackInstanceName.trim() || selectedMod.title,
        memory: modpackMemory,
        icon: selectedMod.icon_url || null
      });
      showNotification(`"${inst.name}" instance sikeresen létrehozva a modpackből!`, 'success');
      await loadInstances();
      setTimeout(() => {
        showModpackProgressModal = false;
        closeAllModals();
        goto(`/instance/${inst.id}`);
      }, 1000);
    } catch (err) {
      showNotification(`Hiba a modpack telepítésekor: ${err}`, 'error');
      showModpackProgressModal = false;
    }
  }

  async function handleInstallModpackExisting() {
    if (!selectedMod || !selectedVersion || !modpackTargetExisting) return;
    showModpackExistingModal = false;
    showModpackProgressModal = true;
    modpackProgress = { step: 'Előkészítés...', current: 0, total: 100, file_name: '', percent: 1, speed: 0, total_bytes: 0, current_bytes: 0, remaining_time: '' };

    try {
      await invoke('install_modpack_existing_instance', {
        instanceId: modpackTargetExisting.id,
        versionId: selectedVersion.id
      });
      showNotification(`Modpack sikeresen telepítve a(z) "${modpackTargetExisting.name}" instance-be!`, 'success');
      await loadInstances();
      setTimeout(() => {
        showModpackProgressModal = false;
        closeAllModals();
        goto(`/instance/${modpackTargetExisting!.id}`);
      }, 1000);
    } catch (err) {
      showNotification(`Hiba a modpack telepítésekor: ${err}`, 'error');
      showModpackProgressModal = false;
    }
  }

  function selectInstance(inst: Instance) {
    targetInstance = inst;
    let compat: ModrinthVersion[];
    if (isCurrentContentPack) {
      compat = allVersions.filter((v) => v.game_versions.includes(inst.mcVersion));
      if (compat.length === 0) compat = allVersions;
    } else {
      compat = allVersions.filter(
        (v) =>
          v.game_versions.includes(inst.mcVersion) &&
          v.loaders.some((l) => l.toLowerCase() === inst.loader.toLowerCase())
      );
    }
    selectedVersion = compat.length > 0 ? compat[0] : null;
    closeSubModal();
  }

  async function handleInstallClick() {
    if (!selectedVersion || !targetInstance) return;
    installing = true;
    try {
      const deps: DependencyInfo[] = await invoke('get_mod_dependencies', {
        versionId: selectedVersion.id
      });
      if (deps.length > 0) {
        dependencies = deps;
        selectedOptionalDeps = deps
          .filter((d) => d.dependency_type === 'optional')
          .map((d) => d.project.id);
        showDepModal = true;
        installing = false;
      } else {
        await executeInstall(String(selectedVersion.id));
      }
    } catch (err) {
      showNotification(`Hiba: ${err}`, 'error');
      installing = false;
    }
  }

  async function executeInstall(versionId: string, installDeps: boolean = false) {
    if (!targetInstance) return;
    installing = true;
    try {
      if (installDeps && dependencies.length > 0) {
        const toInstall = dependencies.filter(
          (d) => d.dependency_type === 'required' || selectedOptionalDeps.includes(d.project.id)
        );

        for (const dep of toInstall) {
          const depVersions: ModrinthVersion[] = await invoke('get_mod_versions', {
            projectId: dep.project.id,
            loader: targetInstance.loader,
            gameVersion: targetInstance.mcVersion
          });
          if (depVersions.length > 0) {
            await invoke('install_mod', {
              instanceId: targetInstance.id,
              instancePath: targetInstance.gameDir,
              versionId: depVersions[0].id,
              projectType: 'mod'
            });
          }
        }
      }
      await invoke('install_mod', {
        instanceId: targetInstance.id,
        instancePath: targetInstance.gameDir,
        versionId: versionId,
        projectType: selectedMod?.project_type || projectType
      });
      showNotification(`${selectedMod?.title} sikeresen telepítve!`, 'success');
      closeAllModals();
    } catch (err) {
      showNotification(`Telepítési hiba: ${err}`, 'error');
    } finally {
      installing = false;
    }
  }

  function closeAllModals() {
    isClosingModal = true;
    setTimeout(() => {
      showInfoModal = false;
      showInstancePicker = false;
      showVersionPicker = false;
      showDepModal = false;
      showModpackNewModal = false;
      showModpackExistingModal = false;
      targetInstance = null;
      selectedVersion = null;
      dependencies = [];
      installing = false;
      isClosingModal = false;
    }, 300);
  }

  function closeSubModal() {
    isClosingSubModal = true;
    setTimeout(() => {
      showInstancePicker = false;
      showVersionPicker = false;
      showDepModal = false;
      installing = false;
      isClosingSubModal = false;
    }, 300);
  }

  let isCurrentContentPack = $derived(
    selectedMod?.project_type === 'resourcepack' ||
    selectedMod?.project_type === 'datapack' ||
    projectType === 'resourcepack' ||
    projectType === 'datapack'
  );

  let compatibleVersionsForTarget = $derived.by(() => {
    if (!targetInstance || !allVersions.length) return [];
    if (isCurrentContentPack) {
      const matched = allVersions.filter((v) => v.game_versions.includes(targetInstance!.mcVersion));
      return matched.length > 0 ? matched : allVersions;
    }
    return allVersions.filter(
      (v) =>
        v.game_versions.includes(targetInstance!.mcVersion) &&
        v.loaders.some((l) => l.toLowerCase() === targetInstance!.loader.toLowerCase())
    );
  });

  function toggleOptionalDep(id: string) {
    if (selectedOptionalDeps.includes(id)) {
      selectedOptionalDeps = selectedOptionalDeps.filter((i) => i !== id);
    } else {
      selectedOptionalDeps = [...selectedOptionalDeps, id];
    }
  }
</script>

<svelte:window onclick={handleWindowClick} />

<div class="mod-browser-layout">
  <div class="mod-browser-main-container">
    <div class="mod-browser-header-centered">
      <h1>
        {projectType === 'mod' ? 'Mod Böngésző' :
         projectType === 'modpack' ? 'Modpack Böngésző' :
         projectType === 'resourcepack' ? 'Resource Pack Böngésző' :
         'Datapack Böngésző'}
      </h1>
      <p>
        {projectType === 'mod' ? 'Találd meg és telepítsd a legjobb kiegészítőket' :
         projectType === 'modpack' ? 'Fedezz fel komplett modpackeket egyetlen kattintással' :
         projectType === 'resourcepack' ? 'Szabd testre a textúrákat, hangokat és modelleket' :
         'Bővítsd a világot, játékmechanikákat és struktúrákat datapackekkel'}
      </p>

      <div class="project-type-toggle">
        <button
          type="button"
          class={`type-pill ${projectType === 'mod' ? 'active' : ''}`}
          onclick={() => switchProjectType('mod')}
        >
          <Package size={15} />
          <span>Modok</span>
        </button>
        <button
          type="button"
          class={`type-pill ${projectType === 'modpack' ? 'active' : ''}`}
          onclick={() => switchProjectType('modpack')}
        >
          <Layers size={15} />
          <span>Modpackek</span>
        </button>
        <button
          type="button"
          class={`type-pill ${projectType === 'resourcepack' ? 'active' : ''}`}
          onclick={() => switchProjectType('resourcepack')}
        >
          <Palette size={15} />
          <span>Resource Packek</span>
        </button>
        <button
          type="button"
          class={`type-pill ${projectType === 'datapack' ? 'active' : ''}`}
          onclick={() => switchProjectType('datapack')}
        >
          <Database size={15} />
          <span>Datapackek</span>
        </button>
      </div>

      <div class="search-bar-hero">
        <Search class="search-icon" size={22} />
        <input
          type="text"
          placeholder={
            projectType === 'mod' ? 'Keress modokat (pl. Sodium, Iris...)' :
            projectType === 'modpack' ? 'Keress modpackeket (pl. Fabulously Optimized, Better MC...)' :
            projectType === 'resourcepack' ? 'Keress resource packokat (pl. Fresh Animations, Bare Bones...)' :
            'Keress datapackeket (pl. Terralith, Incendium...)'
          }
          bind:value={searchQuery}
          onkeydown={(e) => e.key === 'Enter' && handleSearch(1)}
        />
        <button class="search-confirm-btn" onclick={() => handleSearch(1)} disabled={loading}>
          {#if loading}
            <Loader2 class="spin" size={20} />
          {:else}
            Keresés
          {/if}
        </button>
      </div>
    </div>

    <div class="mod-browser-content-scrollable custom-scrollbar" bind:this={scrollContainer}>
      <div class="mods-grid">
        {#if loading && mods.length === 0}
          {#each Array(12) as _, i}
            <div class="mod-card skeleton">
              <div class="mod-card-icon skeleton-box"></div>
              <div class="mod-card-details">
                <div class="skeleton-line title"></div>
                <div class="skeleton-line text"></div>
              </div>
            </div>
          {/each}
        {:else}
          {#each mods as mod, index (mod.project_id)}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="mod-card staggered-fade-in"
              style={`animation-delay: ${index * 0.05}s`}
              onclick={() => openModInfo(mod)}
            >
              <div class="mod-card-icon">
                {#if mod.icon_url}
                  <img src={mod.icon_url} alt="" />
                {:else if mod.project_type === 'resourcepack'}
                  <Palette size={24} />
                {:else if mod.project_type === 'datapack'}
                  <Database size={24} />
                {:else}
                  <Package size={24} />
                {/if}
              </div>
              <div class="mod-card-details">
                <div class="mod-card-top">
                  <h3 class="truncate-text">{mod.title}</h3>
                  {#if mod.project_type === 'modpack'}
                    <span class="side-badge modpack">Modpack</span>
                  {:else if mod.project_type === 'resourcepack'}
                    <span class="side-badge resourcepack">Resource Pack</span>
                  {:else if mod.project_type === 'datapack'}
                    <span class="side-badge datapack">Datapack</span>
                  {:else if mod.client_side === 'required' && mod.server_side === 'required'}
                    <span class="side-badge both">Client & Server</span>
                  {:else if mod.client_side === 'required'}
                    <span class="side-badge client">Client Side</span>
                  {:else if mod.server_side === 'required'}
                    <span class="side-badge server">Server Side</span>
                  {:else if mod.client_side === 'optional' && mod.server_side === 'optional'}
                    <span class="side-badge both">Universal</span>
                  {:else if mod.client_side === 'optional'}
                    <span class="side-badge client">Client (Opt)</span>
                  {:else if mod.server_side === 'optional'}
                    <span class="side-badge server">Server (Opt)</span>
                  {/if}
                </div>
                <p class="mod-desc-short">{mod.description}</p>
                <div class="mod-card-footer">
                  <div class="mod-stat-min">
                    <Download size={14} />
                    <span>{mod.downloads.toLocaleString()}</span>
                  </div>
                </div>
              </div>
            </div>
          {/each}
        {/if}
      </div>

      {#if totalPages > 1}
        <div class="mod-browser-pagination">
          <button
            type="button"
            class="pagination-nav-btn"
            disabled={currentPage <= 1 || loading}
            onclick={() => goToPage(currentPage - 1)}
            title="Előző oldal"
          >
            <ChevronLeft size={16} />
            <span>Előző</span>
          </button>

          <div class="pagination-page-numbers">
            {#each visiblePages as p}
              {#if p === '...'}
                <span class="pagination-ellipsis">...</span>
              {:else}
                <button
                  type="button"
                  class={`pagination-number-btn ${currentPage === p ? 'active' : ''}`}
                  disabled={loading}
                  onclick={() => goToPage(Number(p))}
                >
                  {p}
                </button>
              {/if}
            {/each}
          </div>

          <button
            type="button"
            class="pagination-nav-btn"
            disabled={currentPage >= totalPages || loading}
            onclick={() => goToPage(currentPage + 1)}
            title="Következő oldal"
          >
            <span>Következő</span>
            <ChevronRight size={16} />
          </button>
        </div>
      {/if}
    </div>

    <!-- FIXED STABLE BOTTOM FILTER DOCK -->
    <div class="mod-browser-bottom-bar">
      <div
        class="filter-dock-pill"
        onwheel={(e) => {
          if (!showLoaderDropdown && !showVersionDropdown && scrollContainer) {
            scrollContainer.scrollTop += e.deltaY;
          }
        }}
      >
        <div class="filter-controls-group">
          <div class="filter-label">
            <Filter size={15} />
            <span>Szűrők:</span>
          </div>

          <!-- LOADER DROPDOWN (Only for Mods and Modpacks) -->
          {#if projectType === 'mod' || projectType === 'modpack'}
            <div class="filter-dropdown-wrapper">
              <button
                type="button"
                class={`filter-dropdown-btn ${filterLoader !== 'all' ? 'has-value' : ''} ${showLoaderDropdown ? 'open' : ''}`}
                onclick={(e) => {
                  e.stopPropagation();
                  showVersionDropdown = false;
                  showLoaderDropdown = !showLoaderDropdown;
                }}
              >
                {#if selectedLoaderObj?.icon}
                  <img src={selectedLoaderObj.icon} alt="" class="btn-loader-icon" />
                {:else}
                  <Layers size={16} />
                {/if}
                <span class="btn-text">{selectedLoaderObj?.name || 'Összes loader'}</span>
                <ChevronDown size={14} class={`chevron-icon ${showLoaderDropdown ? 'rotate' : ''}`} />
              </button>

              {#if showLoaderDropdown}
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div class="filter-dropdown-menu upward-menu" onclick={(e) => e.stopPropagation()}>
                  <div class="dropdown-header-title">Mod Loader (Fork)</div>
                  {#each LOADERS as l (l.id)}
                    <button
                      type="button"
                      class={`dropdown-item ${filterLoader === l.id ? 'active' : ''}`}
                      onclick={() => selectLoader(l.id)}
                    >
                      <div class="item-left">
                        {#if l.icon}
                          <img src={l.icon} alt="" class="dropdown-loader-icon" />
                        {:else}
                          <Layers size={18} class="dropdown-default-icon" />
                        {/if}
                        <span>{l.name}</span>
                      </div>
                      {#if filterLoader === l.id}
                        <Check size={16} class="check-icon" />
                      {/if}
                    </button>
                  {/each}
                </div>
              {/if}
            </div>
          {/if}

          <!-- VERSION DROPDOWN -->
          <div class="filter-dropdown-wrapper">
            <button
              type="button"
              class={`filter-dropdown-btn ${filterVersion !== 'all' ? 'has-value' : ''} ${showVersionDropdown ? 'open' : ''}`}
              onclick={(e) => {
                e.stopPropagation();
                showLoaderDropdown = false;
                showVersionDropdown = !showVersionDropdown;
              }}
            >
              <img src="/loaders/vanilla.svg" alt="" class="btn-loader-icon" />
              <span class="btn-text">{filterVersion === 'all' ? 'Összes verzió' : `MC ${filterVersion}`}</span>
              <ChevronDown size={14} class={`chevron-icon ${showVersionDropdown ? 'rotate' : ''}`} />
            </button>

            {#if showVersionDropdown}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div class="filter-dropdown-menu upward-menu version-menu" onclick={(e) => e.stopPropagation()}>
                <div class="dropdown-header-title">Minecraft Verzió</div>
                <div class="version-search-box">
                  <Search size={14} />
                  <input
                    type="text"
                    placeholder="Keresés (pl. 1.21)..."
                    bind:value={versionSearchQuery}
                  />
                </div>
                <div class="version-items-scroll">
                  <button
                    type="button"
                    class={`dropdown-item ${filterVersion === 'all' ? 'active' : ''}`}
                    onclick={() => selectFilterVersion('all')}
                  >
                    <div class="item-left">
                      <Sparkles size={16} class="dropdown-default-icon" />
                      <span>Összes verzió</span>
                    </div>
                    {#if filterVersion === 'all'}
                      <Check size={16} class="check-icon" />
                    {/if}
                  </button>
                  {#each filteredVersions as ver (ver)}
                    <button
                      type="button"
                      class={`dropdown-item ${filterVersion === ver ? 'active' : ''}`}
                      onclick={() => selectFilterVersion(ver)}
                    >
                      <div class="item-left">
                        <span class="version-pill-badge">{ver}</span>
                      </div>
                      {#if filterVersion === ver}
                        <Check size={16} class="check-icon" />
                      {/if}
                    </button>
                  {/each}
                </div>
              </div>
            {/if}
          </div>

          <!-- CLEAR FILTERS -->
          {#if filterLoader !== 'all' || filterVersion !== 'all' || activeCategory !== 'all'}
            <button type="button" class="clear-filters-btn" onclick={clearFilters} title="Szűrők visszaállítása">
              <RotateCcw size={14} />
              <span>Szűrők törlése</span>
            </button>
          {/if}
        </div>

        <div class="filter-dock-divider"></div>

        <div class="filter-meta-right">
          <div class="results-badge" title={`${totalHits.toLocaleString()} elem összesen`}>
            <Package size={14} />
            <span>
              {loading ? 'Keresés...' :
               projectType === 'mod' ? `${totalHits.toLocaleString()} mod` :
               projectType === 'modpack' ? `${totalHits.toLocaleString()} modpack` :
               projectType === 'resourcepack' ? `${totalHits.toLocaleString()} resource pack` :
               `${totalHits.toLocaleString()} datapack`}
            </span>
          </div>

          {#if totalPages > 1}
            <div class="dock-quick-page">
              <button
                type="button"
                class="dock-page-arrow"
                disabled={currentPage <= 1 || loading}
                onclick={() => goToPage(currentPage - 1)}
                title="Előző oldal"
              >
                <ChevronLeft size={14} />
              </button>
              <span class="dock-page-indicator">{currentPage} / {totalPages}</span>
              <button
                type="button"
                class="dock-page-arrow"
                disabled={currentPage >= totalPages || loading}
                onclick={() => goToPage(currentPage + 1)}
                title="Következő oldal"
              >
                <ChevronRight size={14} />
              </button>
            </div>
          {/if}
        </div>
      </div>
    </div>
  </div>

  <aside class="mod-browser-right-sidebar" onscroll={handleCategoryMouseLeave}>
    {#each CATEGORIES as cat (cat.id)}
      {@const Icon = cat.icon}
      <button
        type="button"
        class={`cat-sidebar-btn ${activeCategory === cat.id ? 'active' : ''}`}
        onclick={() => selectCategory(cat.id)}
        onmouseenter={(e) => handleCategoryMouseEnter(e, cat.name)}
        onmouseleave={handleCategoryMouseLeave}
        aria-label={cat.name}
      >
        <Icon size={20} />
      </button>
    {/each}
  </aside>

  {#if hoveredCategory}
    <div
      class="cat-floating-tooltip"
      style={`top: ${hoveredCategory.y}px; right: ${hoveredCategory.right}px;`}
    >
      <span>{hoveredCategory.name}</span>
    </div>
  {/if}

  {#if showInfoModal && selectedMod}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class={`create-modal-overlay ${isClosingModal ? 'closing' : ''}`}
      use:portal
      onclick={closeAllModals}
    >
      <div class="create-modal-content mod-info-modal-fixed" onclick={(e) => e.stopPropagation()}>
        <button class="close-btn" onclick={closeAllModals} aria-label="Close"><X size={20} /></button>
        <div class="mod-info-header-modal">
          <div class="mod-info-icon-box">
            {#if selectedMod.icon_url}
              <img src={selectedMod.icon_url} alt="" />
            {:else}
              <Package size={40} />
            {/if}
          </div>
          <div class="mod-info-titles">
            <h2 class="truncate-text">{selectedMod.title}</h2>
            <div class="author-tag">
              <User size={14} /> <span>{selectedMod.author}</span>
            </div>
          </div>
        </div>
        <div class="mod-info-stats-modal">
          <div class="modal-stat-pill">
            <Download size={18} /> <span>{selectedMod.downloads.toLocaleString()} Letöltés</span>
          </div>
          <div class="modal-stat-pill">
            <Box size={18} /> <span>{
              selectedMod.project_type === 'resourcepack' ? 'Resource Pack' :
              selectedMod.project_type === 'datapack' ? 'Datapack' :
              selectedMod.project_type === 'modpack' ? 'Modpack' : 'Mod'
            }</span>
          </div>
        </div>
        <div class="mod-info-body-modal">
          {#if selectedMod.project_type === 'modpack' || projectType === 'modpack'}
            <div class="modal-section">
              <div class="modal-section-header">
                <h3>Modpack Verzió</h3>
                <button class="change-link-btn" onclick={() => (showVersionPicker = true)}>
                  <List size={14} /> <span>Összes verzió</span>
                </button>
              </div>
              {#if selectedVersion}
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div class="selection-pill-card selectable" onclick={() => (showVersionPicker = true)}>
                  <div class="p-icon version-icon-box"><Sparkles size={18} /></div>
                  <div class="p-text">
                    <span class="p-name">{selectedVersion.name || selectedVersion.version_number}</span>
                    <div class="p-badges">
                      {#if selectedVersion.game_versions && selectedVersion.game_versions.length > 0}
                        <span class="p-badge-ver">MC {selectedVersion.game_versions[0]}</span>
                      {/if}
                      {#if selectedVersion.loaders && selectedVersion.loaders.length > 0}
                        <span class="p-badge-loader">{selectedVersion.loaders.join(', ')}</span>
                      {/if}
                    </div>
                  </div>
                  <div class="p-badge-main recommended">Kiválasztva</div>
                </div>
              {:else}
                <div class="modal-alert-info">
                  <Info size={18} /> <span>Verziók betöltése...</span>
                </div>
              {/if}
            </div>

            <div class="modal-section">
              <div class="modal-section-header">
                <h3>Hova szeretnéd telepíteni?</h3>
              </div>
              <div class="modpack-action-cards">
                <button
                  type="button"
                  class="modpack-action-choice-btn primary"
                  disabled={!selectedVersion}
                  onclick={() => {
                    modpackInstanceName = selectedMod?.title || '';
                    showModpackNewModal = true;
                  }}
                >
                  <div class="choice-icon-wrap green">
                    <Plus size={22} />
                  </div>
                  <div class="choice-text">
                    <h4>Új instance létrehozása</h4>
                    <p>Önálló profilként, automatikus Minecraft & Loader beállításokkal</p>
                  </div>
                  <ChevronRight size={18} class="choice-arrow" />
                </button>

                <button
                  type="button"
                  class="modpack-action-choice-btn secondary"
                  disabled={!selectedVersion || instances.length === 0}
                  onclick={() => {
                    showModpackExistingModal = true;
                  }}
                >
                  <div class="choice-icon-wrap blue">
                    <Folder size={22} />
                  </div>
                  <div class="choice-text">
                    <h4>Telepítés létező instance-be</h4>
                    <p>Modok és konfigurációk berakása egy meglévő profilodba</p>
                  </div>
                  <ChevronRight size={18} class="choice-arrow" />
                </button>
              </div>
            </div>
          {:else}
            <div class="modal-section">
              <div class="modal-section-header">
                <h3>Cél Instance</h3>
                <button class="change-link-btn" onclick={() => (showInstancePicker = true)}>
                  <List size={14} /> <span>Összes</span>
                </button>
              </div>
              {#if targetInstance}
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div class="selection-pill-card selectable" onclick={() => (showInstancePicker = true)}>
                  <div class="p-icon">
                    {#if targetInstance.icon}
                      <img src={targetInstance.icon} alt="" />
                    {:else}
                      <Folder size={20} />
                    {/if}
                  </div>
                  <div class="p-text">
                    <span class="p-name">{targetInstance.name}</span>
                    <div class="p-badges">
                      <span class="p-badge-ver">{targetInstance.mcVersion}</span>
                      <span class="p-badge-loader">{targetInstance.loader}</span>
                    </div>
                  </div>
                  <div class="p-badge-main recommended">Most Played</div>
                </div>
              {:else}
                <div class="modal-alert-error">
                  <AlertCircle size={18} /> <span>Nincs kompatibilis instance.</span>
                </div>
              {/if}
            </div>
            <div class="modal-section">
              <div class="modal-section-header">
                <h3>Választott Verzió</h3>
                {#if targetInstance}
                  <button class="change-link-btn" onclick={() => (showVersionPicker = true)}>
                    <List size={14} /> <span>Összes</span>
                  </button>
                {/if}
              </div>
              {#if selectedVersion}
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div class="selection-pill-card selectable" onclick={() => (showVersionPicker = true)}>
                  <div class="p-icon version-icon-box"><Sparkles size={18} /></div>
                  <div class="p-text">
                    <span class="p-name">{selectedVersion.version_number}</span>
                    <div class="p-badges">
                      <span class="p-badge-date">{new Date(String(selectedVersion.date_published)).toLocaleDateString()}</span>
                    </div>
                  </div>
                </div>
              {:else}
                <div class="modal-alert-info">
                  <Info size={18} /> <span>Válassz egy verziót...</span>
                </div>
              {/if}
            </div>
          {/if}
        </div>
        {#if selectedMod.project_type !== 'modpack' && projectType !== 'modpack'}
          <div class="modal-footer-sticky">
            <button
              class="modal-install-confirm-btn"
              onclick={handleInstallClick}
              disabled={!selectedVersion || installing}
            >
              {#if installing}
                <Loader2 class="spin" size={20} />
              {:else}
                <Download size={20} />
              {/if}
              <span>{
                installing ? 'Telepítés...' :
                selectedMod.project_type === 'resourcepack' || projectType === 'resourcepack' ? 'Resource Pack Telepítése' :
                selectedMod.project_type === 'datapack' || projectType === 'datapack' ? 'Datapack Telepítése' :
                'Mod Telepítése'
              }</span>
            </button>
          </div>
        {/if}
      </div>
    </div>
  {/if}

  {#if showInstancePicker || showVersionPicker || showDepModal}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class={`create-modal-overlay sub-modal-overlay ${isClosingSubModal ? 'closing' : ''}`}
      use:portal
      onclick={closeSubModal}
    >
      <div class="create-modal-content picker-modal-fixed" onclick={(e) => e.stopPropagation()}>
        <button class="close-btn" onclick={closeSubModal} aria-label="Close"><X size={20} /></button>
        <div class="picker-header-modal">
          <h2>
            {showInstancePicker ? 'Válassz Instance-t' : showVersionPicker ? 'Válassz Verziót' : 'Függőségek'}
          </h2>
          <p>
            {showInstancePicker ? (isCurrentContentPack ? 'Válassz instance-t a telepítéshez' : 'Csak modolható verziók') : showVersionPicker ? `Kompatibilis: ${targetInstance?.mcVersion}` : 'Szükséges kiegészítők'}
          </p>
        </div>
        <div class="picker-body-scrollable">
          {#if showInstancePicker}
            {#each (isCurrentContentPack ? instances : instances.filter((i) => i.loader.toLowerCase() !== 'vanilla')) as inst (inst.id)}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div class="picker-option-card" onclick={() => selectInstance(inst)}>
                <div class="opt-icon">
                  {#if inst.icon}
                    <img src={inst.icon} alt="" />
                  {:else}
                    <Folder size={24} />
                  {/if}
                </div>
                <div class="opt-info">
                  <span class="opt-name truncate-text">{inst.name}</span>
                  <div class="opt-meta-badges">
                    <span class="opt-badge-ver">{inst.mcVersion}</span>
                    <span class="opt-badge-loader">{inst.loader}</span>
                  </div>
                </div>
                <ChevronRight size={20} class="opt-arrow" />
              </div>
            {/each}
          {/if}

          {#if showVersionPicker}
            {#each (selectedMod?.project_type === 'modpack' || projectType === 'modpack' ? allVersions : compatibleVersionsForTarget) as v (v.id)}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div
                class="picker-option-card"
                onclick={() => {
                  selectedVersion = v;
                  closeSubModal();
                }}
              >
                <div class="opt-icon version-opt"><Sparkles size={20} /></div>
                <div class="opt-info">
                  <span class="opt-name">{v.name || v.version_number}</span>
                  <div class="opt-meta-badges">
                    {#if v.game_versions && v.game_versions.length > 0}
                      <span class="opt-badge-ver">MC {v.game_versions[0]}</span>
                    {/if}
                    {#if v.loaders && v.loaders.length > 0}
                      <span class="opt-badge-loader">{v.loaders.join(', ')}</span>
                    {/if}
                    <span class="opt-badge-date">{new Date(String(v.date_published)).toLocaleDateString()}</span>
                  </div>
                </div>
                <ChevronRight size={20} class="opt-arrow" />
              </div>
            {/each}
          {/if}

          {#if showDepModal}
            <div class="dependency-list">
              {#each dependencies as dep (dep.project.id)}
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div
                  class={`dep-item ${dep.dependency_type}`}
                  onclick={() => dep.dependency_type === 'optional' && toggleOptionalDep(dep.project.id)}
                >
                  <div class="dep-icon">
                    <img src={dep.project.icon_url || ''} alt="" />
                  </div>
                  <div class="dep-info">
                    <span class="dep-name">{dep.project.title}</span>
                    <span class={`dep-type-tag ${dep.dependency_type}`}>
                      {dep.dependency_type === 'required' ? 'Kötelező' : 'Ajánlott'}
                    </span>
                  </div>
                  {#if dep.dependency_type === 'optional'}
                    <div class={`dep-checkbox ${selectedOptionalDeps.includes(dep.project.id) ? 'checked' : ''}`}>
                      {#if selectedOptionalDeps.includes(dep.project.id)}
                        <Check size={14} />
                      {/if}
                    </div>
                  {/if}
                  {#if dep.dependency_type === 'required'}
                    <Check size={18} class="dep-check-fixed" />
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
        </div>

        {#if showDepModal}
          <div class="picker-footer-modal dual-btns">
            <button
              class="modal-install-confirm-btn dep-confirm-btn"
              onclick={() => executeInstall(String(selectedVersion?.id || ''), true)}
              disabled={installing}
            >
              {#if installing}
                <Loader2 class="spin" size={18} />
              {:else}
                <Download size={18} />
              {/if}
              <span>Kijelöltek telepítése</span>
            </button>
            <button
              class="btn-secondary-outline only-main-btn"
              onclick={() => executeInstall(String(selectedVersion?.id || ''), false)}
              disabled={installing}
            >
              Csak a fő modot
            </button>
          </div>
        {/if}
      </div>
    </div>
  {/if}

  {#if showModpackNewModal && selectedMod && selectedVersion}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="create-modal-overlay sub-modal-overlay" use:portal onclick={() => (showModpackNewModal = false)}>
      <div class="create-modal-content modpack-dialog-fixed" onclick={(e) => e.stopPropagation()}>
        <button class="close-btn" onclick={() => (showModpackNewModal = false)} aria-label="Bezárás"><X size={18} /></button>
        <div class="picker-header-modal">
          <h2>Új Instance Létrehozása</h2>
          <p>A modpack konfigurációja automatikusan beállításra kerül</p>
        </div>

        <div class="modpack-dialog-body">
          <div class="modpack-summary-row">
            <div class="dialog-pack-icon-wrap">
              {#if selectedMod.icon_url}
                <img src={selectedMod.icon_url} alt="" class="dialog-pack-icon" />
              {:else}
                <Package size={28} />
              {/if}
            </div>
            <div class="dialog-pack-info">
              <h4>{selectedMod.title}</h4>
              <div class="dialog-pack-badges">
                {#if selectedVersion.game_versions?.[0]}
                  <span class="dialog-badge mc">Minecraft {selectedVersion.game_versions[0]}</span>
                {/if}
                {#if selectedVersion.loaders?.[0]}
                  <span class="dialog-badge loader">{selectedVersion.loaders[0]}</span>
                {/if}
              </div>
            </div>
          </div>

          <div class="dialog-form-group">
            <label for="modpack-inst-name">Instance Neve</label>
            <input
              id="modpack-inst-name"
              type="text"
              bind:value={modpackInstanceName}
              placeholder="Pl. Fabulously Optimized"
              class="dialog-input"
            />
          </div>

          <div class="dialog-form-group">
            <div class="ram-label-row">
              <label for="modpack-ram-slider">Memória (RAM)</label>
              <span class="ram-value-display">{modpackMemory} MB ({(modpackMemory / 1024).toFixed(1)} GB)</span>
            </div>
            <input
              id="modpack-ram-slider"
              type="range"
              min={1024}
              max={16384}
              step={512}
              bind:value={modpackMemory}
              class="styled-slider"
            />
          </div>
        </div>

        <div class="modal-footer-sticky dual">
          <button
            type="button"
            class="btn-cancel"
            onclick={() => (showModpackNewModal = false)}
          >
            Mégse
          </button>
          <button
            type="button"
            class="modal-install-confirm-btn flex-1"
            onclick={handleInstallModpackNew}
            disabled={!modpackInstanceName.trim()}
          >
            <Download size={18} />
            <span>Létrehozás és Telepítés</span>
          </button>
        </div>
      </div>
    </div>
  {/if}

  {#if showModpackExistingModal && selectedMod && selectedVersion}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="create-modal-overlay sub-modal-overlay" use:portal onclick={() => (showModpackExistingModal = false)}>
      <div class="create-modal-content picker-modal-fixed" onclick={(e) => e.stopPropagation()}>
        <button class="close-btn" onclick={() => (showModpackExistingModal = false)} aria-label="Bezárás"><X size={18} /></button>
        <div class="picker-header-modal">
          <h2>Válassz Instance-t</h2>
          <p>Melyik profilba szeretnéd betölteni a modpack tartalmát?</p>
        </div>

        <div class="picker-body-scrollable">
          {#each instances as inst (inst.id)}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class={`picker-option-card ${modpackTargetExisting?.id === inst.id ? 'active' : ''}`}
              onclick={() => (modpackTargetExisting = inst)}
            >
              <div class="opt-icon">
                {#if inst.icon}
                  <img src={inst.icon} alt="" />
                {:else}
                  <Folder size={24} />
                {/if}
              </div>
              <div class="opt-info">
                <span class="opt-name truncate-text">{inst.name}</span>
                <div class="opt-meta-badges">
                  <span class="opt-badge-ver">{inst.mcVersion}</span>
                  <span class="opt-badge-loader">{inst.loader}</span>
                </div>
              </div>
              {#if modpackTargetExisting?.id === inst.id}
                <Check size={18} class="opt-check-icon" />
              {/if}
            </div>
          {/each}
        </div>

        <div class="modal-footer-sticky dual">
          <button
            type="button"
            class="btn-cancel"
            onclick={() => (showModpackExistingModal = false)}
          >
            Mégse
          </button>
          <button
            type="button"
            class="modal-install-confirm-btn flex-1"
            onclick={handleInstallModpackExisting}
            disabled={!modpackTargetExisting}
          >
            <Download size={18} />
            <span>Telepítés a profilba</span>
          </button>
        </div>
      </div>
    </div>
  {/if}

  {#if showModpackProgressModal}
    <div class="create-modal-overlay progress-modal-overlay" use:portal>
      <div class="create-modal-content progress-modal-fixed">
        <div class="progress-modal-header">
          <div class="progress-icon-ring">
            <Loader2 class="spin" size={32} />
          </div>
          <h2>Modpack Telepítése...</h2>
          <p class="progress-step-text">{modpackProgress.step || 'Fájlok előkészítése...'}</p>
        </div>

        <div class="progress-bar-wrapper">
          <div class="progress-bar-track">
            <div
              class="progress-bar-fill"
              style={`width: ${Math.min(100, Math.max(0, modpackProgress.percent))}%;`}
            ></div>
          </div>
          <div class="progress-meta-row">
            <span class="progress-filename truncate-text" title={modpackProgress.file_name}>
              {modpackProgress.file_name || 'Kérlek várj...'}
            </span>
            <span class="progress-pct">{Math.round(modpackProgress.percent)}%</span>
          </div>
          {#if modpackProgress.speed > 0 || (modpackProgress.remaining_time && modpackProgress.remaining_time !== '0 mp')}
            <div class="progress-extra-meta">
              {#if modpackProgress.speed > 0}
                <span class="progress-speed"><Zap size={12} /> {modpackProgress.speed.toFixed(1)} MB/s</span>
              {/if}
              {#if modpackProgress.remaining_time && modpackProgress.remaining_time !== '0 mp'}
                <span class="progress-eta"><Clock size={12} /> {modpackProgress.remaining_time}</span>
              {/if}
            </div>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</div>
