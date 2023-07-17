<script lang="ts">
	import panzoom from 'panzoom';
	import PersonCard from '$lib/PersonCard.svelte';
	import { grid } from './store';
	import { onDestroy } from 'svelte';
	import Connector from './Connector.svelte';

	// panzoom
	function initPanzoom(node: HTMLElement) {
		panzoom(node);
	}

	// grid
	let gridColumns = 0;
	let gridRows = 0;
	let unsubscribe = grid.subscribe((grid) => {
		gridRows = grid.length;
		if (grid.length > 0) {
			gridColumns = grid[0].length;
		}
	});
	onDestroy(unsubscribe);
</script>

<div
	class="tree-view"
	style="grid-template-columns: repeat({gridColumns}, 200px); grid-template-rows: repeat({gridRows}, 200px)"
	use:initPanzoom
>
	{#each $grid as layer}
		{#each layer as item}
			{#if item.isPerson()}
				<PersonCard person={item.getPerson()} />
			{:else}
				<Connector connections={item.getConnections()} />
			{/if}
		{/each}
	{/each}
</div>

<style>
	.tree-view {
		user-select: none;
		cursor: move;
		display: grid;
        place-content: center;
	}
</style>
