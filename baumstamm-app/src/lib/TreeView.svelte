<script lang="ts">
	import panzoom from 'panzoom';
	import PersonCard from '$lib/PersonCard.svelte';
	import { grid, persons } from './store';
	import { onDestroy, onMount } from 'svelte';
	import Connector from './Connector.svelte';
	import type { GridItem, PersonId } from '../bindings';
	import type { Person } from './Person';
	import type { Unsubscriber } from 'svelte/store';

	// panzoom
	function initPanzoom(node: HTMLElement) {
		panzoom(node);
	}

	// grid
	let unsubscribers: Unsubscriber[] = [];

	let gridColumns = 0;
	let gridRows = 0;
	let personStore: Person[] = [];

	onMount(() => {
		const gridUnsubscribe = grid.subscribe((grid) => {
			gridRows = grid.length;
			if (grid.length > 0) {
				gridColumns = grid[0].length;
			}
		});
		const personsUnsubscribe = persons.subscribe((store) => (personStore = store));
		unsubscribers.push(gridUnsubscribe);
		unsubscribers.push(personsUnsubscribe);
	});

	onDestroy(() => unsubscribers.forEach((unsubscribe) => unsubscribe()));

	function isPerson(item: GridItem): item is { Person: PersonId } {
		return (item as { Person: PersonId }).Person !== undefined;
	}

	function getPerson(item: { Person: PersonId }): Person {
		const person = personStore.find((person) => person.id == item.Person)!;
		if (person === undefined) {
			throw new Error('Cannot find person');
		}
		return person;
	}
</script>

<div
	class="tree-view"
	style="grid-template-columns: repeat({gridColumns}, 200px); grid-template-rows: repeat({gridRows}, 200px)"
	use:initPanzoom
>
	{#each $grid as layer}
		{#each layer as item}
			{#if isPerson(item)}
				<PersonCard person={getPerson(item)} />
			{:else}
				<Connector connections={item.Connections} />
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
