<script lang="ts">
	import {
		AppShell,
		AppBar,
		Avatar,
		Toast,
		toastStore,
		type ToastSettings
	} from '@skeletonlabs/skeleton';
	import { update, selected } from '$lib/store';
	import Sidebar from '$lib/Sidebar.svelte';
	import TreeView from '$lib/TreeView.svelte';
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
	import { onDestroy, onMount } from 'svelte';

	// tauri events
	let unlisten: UnlistenFn[] = [];
	onMount(() => {
		update();
		listen('open', () => {
			update();
		}).then((handle) => unlisten.push(handle));
		listen('open-error', (e) => {
			const toast: ToastSettings = {
				message: e.payload as string
			};
			toastStore.trigger(toast);
		}).then((handle) => unlisten.push(handle));
		listen('save-as-error', (e) => {
			const toast: ToastSettings = {
				message: e.payload as string
			};
			toastStore.trigger(toast);
		}).then((handle) => unlisten.push(handle));
	});

	onDestroy(() => {
		unlisten.forEach((unlisten) => unlisten());
	});

	// sidebar
	let showSidebar = false;

	function toggleSidebar() {
		showSidebar = !showSidebar;
	}
</script>

<AppShell regionPage="overflow-hidden">
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
						{#if $selected !== null}
							<Avatar initials={$selected.initials()} />
						{:else}
							<div class="placeholder-circle animate-pulse w-20" />
						{/if}
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
				<Sidebar person={$selected} />
			</div>
		{/if}
	</svelte:fragment>

	<!-- body -->
	<TreeView />
</AppShell>

<Toast />
