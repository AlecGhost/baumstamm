<script lang="ts">
	import panzoom from 'panzoom';
	import PersonCard from '$lib/PersonCard.svelte';
	import { personStore } from './store';

	// panzoom
	function initPanzoom(node: HTMLElement) {
		panzoom(node);
	}
</script>

<div class="h-full w-full tree-view" use:initPanzoom>
	{#each $personStore as person}
		<PersonCard {person} />
	{/each}
</div>

<style>
	.tree-view {
		user-select: none;
		position: absolute;
		cursor: move;
	}
</style>
