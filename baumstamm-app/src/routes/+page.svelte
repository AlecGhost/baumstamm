<script>
	import { AppShell, AppBar } from '@skeletonlabs/skeleton';
	import Sidebar from '$lib/Sidebar.svelte';
	let showSidebar = false;
</script>

<AppShell>
	<svelte:fragment slot="header">
		<AppBar>
			<svelte:fragment slot="lead"><h1>Baumstamm</h1></svelte:fragment>
			<svelte:fragment slot="trail">
				<button
					type="button"
					class="btn-icon variant-filled"
					on:click={() => (showSidebar = !showSidebar)}
				/>
			</svelte:fragment>
		</AppBar>
	</svelte:fragment>
	<!-- (sidebarLeft) -->
	<svelte:fragment slot="sidebarRight">
		<div class:hidden={showSidebar} class="h-full">
			<Sidebar />
		</div>
	</svelte:fragment>
	<!-- (pageHeader) -->
	<!-- Router Slot -->
	<slot />
	<!-- ---- / ---- -->
	<!-- <svelte:fragment slot="pageFooter">Page Footer</svelte:fragment> -->
	<!-- (footer) -->
</AppShell>
