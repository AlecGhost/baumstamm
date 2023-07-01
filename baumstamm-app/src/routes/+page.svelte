<script>
	import { AppShell, AppBar, Avatar } from '@skeletonlabs/skeleton';
	import Sidebar from '$lib/Sidebar.svelte';

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

	<slot />
</AppShell>
