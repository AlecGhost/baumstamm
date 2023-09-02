<script lang="ts">
	import {
		AppShell,
		AppBar,
		Avatar,
		Toast,
		toastStore,
		type ToastSettings,
		AppRail,
		AppRailTile
	} from '@skeletonlabs/skeleton';
	import { update, selected } from '$lib/store';
	import Sidebar from '$lib/Sidebar.svelte';
	import TreeView from '$lib/TreeView.svelte';
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
	import { onDestroy, onMount } from 'svelte';
	import DataView from './DataView.svelte';
	import SettingsView from './SettingsView.svelte';

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

	// app rail
	let currentTile: number = 0;

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

	<div class="grid grid-cols-[auto_1fr] h-full w-full">
		<AppRail>
			<AppRailTile bind:group={currentTile} name="tree-view" value={0} title="Tree">
				<i class="fa-solid fa-tree fa-2xl" />
			</AppRailTile>
			<AppRailTile bind:group={currentTile} name="data-view" value={1} title="Data">
				<i class="fa-solid fa-table fa-2xl" />
			</AppRailTile>
			<AppRailTile bind:group={currentTile} name="settings-view" value={2} title="Settings">
				<i class="fa-solid fa-gear fa-2xl" />
			</AppRailTile>
		</AppRail>

		<!-- body -->
		<div class="overflow-hidden">
			<TreeView show={currentTile == 0} />
			<DataView show={currentTile == 1} />
			<SettingsView show={currentTile == 2} />
		</div>
	</div>
</AppShell>

<Toast />
