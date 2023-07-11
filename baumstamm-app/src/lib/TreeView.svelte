<script lang="ts">
	import panzoom from 'panzoom';
	import { Person } from '$lib/Person';
	import PersonCard from '$lib/PersonCard.svelte';
	import { getPersons } from '../bindings';
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
	import { onDestroy, onMount } from 'svelte';

	// tauri events
	let unlisten: UnlistenFn;
	onMount(async () => {
		unlisten = await listen('open', async () => {
			grid = await getGrid();
		});
		grid = await getGrid();
	});

	onDestroy(async () => {
		unlisten();
	});

	// grid
	let grid: Person[] = [];
	async function getGrid(): Promise<Person[]> {
		let persons = await getPersons();
		return persons.map((person) => Person.from(person));
	}

	// panzoom
	function initPanzoom(node: HTMLElement) {
		panzoom(node);
	}
</script>

<div class="h-full w-full tree-view" use:initPanzoom>
	{#each grid as person}
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
