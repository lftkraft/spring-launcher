<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import './SplashScreen.css';

  let { onComplete } = $props<{ onComplete: () => void }>();

  let phase = $state<'bar' | 'circle' | 'fadeout' | 'collapse' | 'done'>('bar');
  let t1: ReturnType<typeof setTimeout> | null = null;
  let t2: ReturnType<typeof setTimeout> | null = null;
  let t3: ReturnType<typeof setTimeout> | null = null;
  let t4: ReturnType<typeof setTimeout> | null = null;

  onMount(() => {
    t1 = setTimeout(() => { phase = 'circle'; }, 2000);
    t2 = setTimeout(() => { phase = 'fadeout'; }, 3000);
    t3 = setTimeout(() => { phase = 'collapse'; }, 4000);
    t4 = setTimeout(() => {
      phase = 'done';
      onComplete();
    }, 5000);
  });

  onDestroy(() => {
    if (t1) clearTimeout(t1);
    if (t2) clearTimeout(t2);
    if (t3) clearTimeout(t3);
    if (t4) clearTimeout(t4);
  });
</script>

<div class={`splash-container ${phase === 'done' ? 'hidden' : ''}`}>
  <div class="splash-content">
    <div
      class={`splash-bar ${phase === 'circle' ? 'circle' : ''} ${phase === 'fadeout' ? 'fadeout' : ''}`}
    ></div>
  </div>
</div>
