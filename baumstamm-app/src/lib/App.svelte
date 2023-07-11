<script lang="ts">
	import {
		AppShell,
		AppBar,
		Avatar,
		Toast,
		toastStore,
		type ToastSettings
	} from '@skeletonlabs/skeleton';
	import Sidebar from '$lib/Sidebar.svelte';
	import TreeView from '$lib/TreeView.svelte';
    import { listen, type UnlistenFn } from '@tauri-apps/api/event';
    import { onDestroy, onMount } from 'svelte';

    // tauri events
    let unlisten: UnlistenFn[] = [];
    onMount(async () => {
        let unlistenOpen = await listen('open-error', (e) => {
            const toast: ToastSettings = {
                message: e.payload as string
            };
            toastStore.trigger(toast);
        });
        let unlistenSave = await listen('save-as-error', (e) => {
            const toast: ToastSettings = {
                message: e.payload as string
            };
            toastStore.trigger(toast);
        });
        unlisten = [unlistenOpen, unlistenSave];
    });

    onDestroy(async () => {
        unlisten.forEach((unlisten) => unlisten());
    });

	// sidebar
	let showSidebar = false;

	function toggleSidebar() {
		showSidebar = !showSidebar;
	}
</script>

<AppShell>
	<svelte:fragment slot="header">
		<AppBar>
			<svelte:fragment slot="lead"><h1>Baumstamm</h1></svelte:fragment>
			<svelte:fragment slot="trail">
				<button
					type="button"
					class="btn btn-icon"
					class:variant-filled={showSidebar}
					on:click={toggleSidebar}
				>
					{#if !showSidebar}
						<Avatar initials="XY" />
					{:else}
						<i class="fa-solid fa-arrow-right" />
					{/if}
				</button>
			</svelte:fragment>
		</AppBar>
	</svelte:fragment>

	<svelte:fragment slot="sidebarRight">
		{#if showSidebar}
			<div class="bg-surface-500/10 w-80 h-full">
				<Sidebar />
			</div>
		{/if}
	</svelte:fragment>

	<!-- body -->
	<TreeView />
</AppShell>

<Toast />
